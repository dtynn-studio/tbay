use std::path::PathBuf;

use clap::Parser;
use humantime::Duration;
use tokio::{signal, time::interval};
use tracing::{debug, error, info, trace, warn_span};

use crate::{
    config::load_config,
    event::{
        Event,
        binance::client::{BnClient, Config, WebsocketControl},
    },
    hub::Hub,
    prelude::*,
    util::term::clean_up_rows,
};

#[derive(Parser)]
pub struct WatchArgs {
    #[arg(long, default_value_t = false)]
    pub dry: bool,

    #[arg(long)]
    pub config: PathBuf,

    #[arg(long, default_value_t = 1000)]
    pub history: usize,

    #[arg(from_global)]
    pub testnet: bool,

    #[arg(from_global)]
    pub proxy: Option<String>,

    #[arg(long, default_value_t = Duration::from(std::time::Duration::from_secs(30)))]
    pub watch: Duration,
}

impl WatchArgs {
    pub async fn run(self) -> Result<()> {
        let _span = warn_span!("watch").entered();
        info!(file=?self.config, "load config");
        let cfg = load_config(self.config)?;
        let mut hub = Hub::default();

        info!("setup hub");
        hub.apply_config(cfg)?;

        let targets = hub.targets();
        info!(?targets, "collected");

        if self.dry {
            info!("dry run, stopped");
            return Ok(());
        }

        let client = BnClient::new(Config {
            testnet: self.testnet,
            proxy: self.proxy,
        })?;

        for target in targets.iter() {
            match client.load_k_history(target).await {
                Ok(ks) => {
                    debug!(s = target.symbol, i = %target.interval, n = ks.len(), "klines loaded");
                    for k in ks {
                        hub.apply_k(k);
                    }
                }
                Err(e) => {
                    error!(t = ?target, "load history: {e:?}");
                }
            }
        }

        let (mut stream, stopper) = client.subscribe_klines(&targets).await?;
        let mut period = interval(self.watch.into());

        let mut state_lines = 0usize;
        let mut sout = std::io::stdout();

        loop {
            tokio::select! {
                evt = stream.recv() => {
                    let Some(evt) = evt else {
                        break;
                    };

                    match evt {
                        Event::K(k) => {
                            hub.apply_k(k);
                        },

                        Event::Disconnect(reason) => {
                            debug!(reason, "event stream disconnected");
                            break;
                        },

                        Event::Broken(reason) => {
                            debug!(reason, "event stream broken");
                        },
                    }
                },

                _ = period.tick() => {
                    trace!("period tick");

                    if state_lines > 0 {
                        _ = clean_up_rows(&mut sout, state_lines as u16);
                    }

                    let mut time_line = 0;
                    if let Ok(t) = OffsetDateTime::now_local() {
                        println!("{t}");
                        time_line = 1;
                    }

                    state_lines = hub.print_state_msgs(true, true) + time_line;
                }

                _ = signal::ctrl_c() => {
                    debug!("signal captured");
                    break;
                },

            }
        }

        _ = stopper.send(WebsocketControl::Disconnect);

        Ok(())
    }
}
