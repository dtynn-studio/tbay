use std::{io::IsTerminal, path::PathBuf};

use clap::Parser;
use crossterm::style::Stylize;
use humantime::Duration;
use time::format_description::well_known::{
    Iso8601,
    iso8601::{
        Config as Iso8601Config, EncodedConfig, FormattedComponents,
        TimePrecision,
    },
};
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

const TIME_CFG: EncodedConfig = Iso8601Config::DEFAULT
    .set_formatted_components(FormattedComponents::DateTime)
    .set_time_precision(TimePrecision::Second {
        decimal_digits: None,
    })
    .encode();

const TIME_FMT: Iso8601<TIME_CFG> = Iso8601::<TIME_CFG>;

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

    #[arg(long, default_value_t = Duration::from(std::time::Duration::from_secs(600)))]
    pub reads: Duration,

    #[arg(long, default_value_t = false)]
    pub disable_notify_reads: bool,
}

impl WatchArgs {
    pub async fn run(self) -> Result<()> {
        let mut sout = std::io::stdout();
        let is_tty = sout.is_terminal();

        let _span = warn_span!("watch").entered();
        info!(file=?self.config, "load config");
        let cfg = load_config(self.config)?;
        let colors = cfg.colors;

        let mut hub = Hub::default();

        info!(tty = is_tty, "setup hub");
        hub.apply_config(cfg, is_tty)?;

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
        let mut watch_period = interval(self.watch.into());
        let mut read_period = interval(self.reads.into());

        info!(watch = %self.watch, reads = %self.reads, "loop start");

        let mut state_lines = 0usize;
        let mut latest_price = None;
        let mut latest_price_color = colors.normal;

        loop {
            tokio::select! {
                evt = stream.recv() => {
                    let Some(evt) = evt else {
                        break;
                    };

                    match evt {
                        Event::K(k) => {
                            let current = k.raw.price_close;
                            if latest_price != Some(current) {
                                let prev = latest_price.replace(current);
                                latest_price_color = match prev {
                                    Some(p) => if current > p {
                                        colors.up
                                    } else if current < p{
                                        colors.down
                                    } else {
                                        colors.normal
                                    },

                                    None => colors.normal,
                                };
                            }

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

                _ = watch_period.tick() => {
                    trace!("watch period tick");

                    if state_lines > 0 {
                        _ = clean_up_rows(&mut sout, state_lines as u16);
                    }

                    state_lines = 0;

                    if let Ok(t) = OffsetDateTime::now_local() && let Ok(f) = t.format(&TIME_FMT) {
                        println!("TIME: {f}");
                        state_lines += 1;
                    }

                    if let Some(price) = latest_price {
                        println!("LATEST PRICE: {}", price.round_dp(2).to_string().with(latest_price_color));
                        state_lines += 1;
                    }

                    state_lines += hub.print_state_msgs(true, true);
                    state_lines += hub.print_read_msgs();
                    state_lines += hub.show_alerts();
                }

                _ = read_period.tick() => {
                    trace!("read period tick");

                    if !self.disable_notify_reads {
                        hub.notify_read_msgs(latest_price);
                    }
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
