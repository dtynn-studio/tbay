use std::path::PathBuf;

use clap::Parser;
use tracing::info;

use crate::{config::load_config, hub::Hub, prelude::*};

#[derive(Parser)]
pub struct WatchArgs {
    #[arg(long, default_value_t = false)]
    pub dry: bool,

    #[arg(long)]
    pub config: PathBuf,
}

impl WatchArgs {
    pub fn run(self) -> Result<()> {
        info!(file=?self.config, "load config");
        let cfg = load_config(self.config)?;
        let mut hub = Hub::default();

        info!("setup normal monitors");

        let mut for_all_pairs = None;
        let mut all_pairs = Vec::new();
        for pair in cfg.pairs.into_iter() {
            if pair.name == "*" {
                for_all_pairs.replace(pair);
                continue;
            }

            all_pairs.push(pair.name.clone());

            let mut for_all_intervals = None;
            let mut all_intervals = Vec::new();

            for interval in pair.intervals {
                let Some(interval_key) = interval.name.as_ref().copied() else {
                    for_all_intervals.replace(interval);
                    continue;
                };

                all_intervals.push(interval_key);

                for monitor in interval.monitors {
                    hub.register_monitor(&pair.name, interval_key, &monitor)?;
                }
            }

            if let Some(for_all_intervals) = for_all_intervals {
                for all_interval_key in all_intervals.iter() {
                    for monitor in for_all_intervals.monitors.iter() {
                        hub.register_monitor(
                            &pair.name,
                            *all_interval_key,
                            monitor,
                        )?;
                    }
                }
            }
        }

        if let Some(for_all_pairs) = for_all_pairs {
            info!("setup * monitors");

            let mut for_all_intervals = None;
            let mut all_intervals = Vec::new();

            for interval in for_all_pairs.intervals {
                let Some(interval_key) = interval.name.as_ref().copied() else {
                    for_all_intervals.replace(interval);
                    continue;
                };

                all_intervals.push(interval_key);

                for monitor in interval.monitors {
                    for sym in all_pairs.iter() {
                        hub.register_monitor(sym, interval_key, &monitor)?;
                    }
                }
            }

            if let Some(for_all_intervals) = for_all_intervals {
                for sym in all_pairs.iter() {
                    for all_interval_key in all_intervals.iter() {
                        for monitor in for_all_intervals.monitors.iter() {
                            hub.register_monitor(
                                sym,
                                *all_interval_key,
                                monitor,
                            )?;
                        }
                    }
                }
            }
        }

        if self.dry {
            info!("dry run, stopped");
            return Ok(());
        }

        Ok(())
    }
}
