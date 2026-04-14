use std::{collections::BTreeMap, io::IsTerminal, path::PathBuf};

use clap::Parser;
use humantime::Duration;
use time::format_description::well_known::{
    Iso8601,
    iso8601::{
        Config as Iso8601Config, EncodedConfig, FormattedComponents,
        TimePrecision,
    },
};
use tokio::{signal, sync::mpsc, time::interval};
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
    web::{Request, serve},
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

    #[arg(long, default_value_t = Duration::from(std::time::Duration::from_secs(10)))]
    pub watch: Duration,

    #[arg(long, default_value_t = Duration::from(std::time::Duration::from_secs(600)))]
    pub reads: Duration,

    #[arg(long, default_value_t = false)]
    pub enable_notify_reads: bool,

    #[arg(long, default_value_t = false)]
    pub enable_server: bool,
}

impl WatchArgs {
    #[cfg(feature = "server")]
    pub async fn run(self) -> Result<()> {
        let mut sout = std::io::stdout();
        let is_tty = sout.is_terminal();

        let _span = warn_span!("watch").entered();
        info!(file=?self.config, "load config");
        let cfg = load_config(self.config)?;

        let mut hub = Hub::default();

        info!(tty = is_tty, "setup hub");
        hub.apply_config(cfg)?;

        let targets = hub.targets();
        info!(?targets, "collected");

        if self.dry {
            info!("dry run, stopped");
            return Ok(());
        }

        let (req_tx, mut req_rx) = mpsc::unbounded_channel();
        if self.enable_server {
            info!("start server");
            tokio::spawn(async { serve(req_tx).await });
        }

        let client = BnClient::new(Config {
            testnet: self.testnet,
            proxy: self.proxy,
        })?;

        let mut latest_price = BTreeMap::new();
        for target in targets.iter() {
            match client.load_k_history(target).await {
                Ok(ks) => {
                    debug!(s = target.symbol, i = %target.interval, n = ks.len(), "klines loaded");
                    for k in ks {
                        latest_price
                            .insert(k.symbol.clone(), k.raw.price_close);
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
        let mut first_watch_tick = true;

        loop {
            tokio::select! {
                evt = stream.recv() => {
                    let Some(evt) = evt else {
                        break;
                    };

                    match evt {
                        Event::K(k) => {
                            let current = k.raw.price_close;
                            latest_price.insert(k.symbol.clone(), current);
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

                req = req_rx.recv() => {
                    let Some(req) = req else {
                        break;
                    };

                    let resp = match req {
                        Request::States(resp_tx) => {
                        },
                    };

                }

                _ = watch_period.tick() => {
                    let is_first = std::mem::replace(&mut first_watch_tick, false);
                    trace!("watch period tick");

                    if state_lines > 0 {
                        _ = clean_up_rows(&mut sout, state_lines as u16);
                    }

                    let mut lines = Vec::new();

                    collect_now_lines(&mut lines);
                    collect_latest_price_lines(&mut lines, &latest_price);
                    hub.collect_state_msgs(&mut lines, is_tty);
                    hub.collect_read_msgs(&mut lines, is_tty);
                    hub.collect_alert_msgs(&mut lines, is_tty, is_first);

                    println!("{}", lines.join("\n"));
                    state_lines = lines.len();
                }

                _ = read_period.tick() => {
                    trace!("read period tick");

                    if self.enable_notify_reads {
                        hub.notify_read_msgs(&latest_price);
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

fn collect_now_lines(lines: &mut Vec<String>) {
    if let Ok(t) = OffsetDateTime::now_local()
        && let Ok(f) = t.format(&TIME_FMT)
    {
        lines.push(format!("TIME: {f}"));
    }
}

fn collect_latest_price_lines(
    lines: &mut Vec<String>,
    latest_prices: &BTreeMap<String, Decimal>,
) {
    if latest_prices.is_empty() {
        return;
    }

    lines.push("LATEST PRICE:".to_owned());
    for (s, p) in latest_prices.iter() {
        lines.push(format!("\t{s}: {}", p.round_dp(2)));
    }
}
