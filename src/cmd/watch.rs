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
        let cfg = load_config(self.config)?;
        let mut hub = Hub::default();

        info!("setup monitors");

        let mut for_all = None;
        let mut all_symbols = Vec::new();
        for symbol in cfg.symbols.into_iter() {
            if symbol.name == "*" {
                for_all.replace(symbol);
                continue;
            }

            all_symbols.push(symbol.name.clone());

            for interval in symbol.intervals {
                for monitor in interval.monitors {
                    hub.register_monitor(
                        &symbol.name,
                        interval.name,
                        &monitor,
                    )?;
                }
            }
        }

        info!("setup * monitors");
        if let Some(for_all) = for_all {
            for interval in for_all.intervals {
                for monitor in interval.monitors {
                    for sym in all_symbols.iter() {
                        hub.register_monitor(sym, interval.name, &monitor)?;
                    }
                }
            }
        }

        if self.dry {
            return Ok(());
        }

        Ok(())
    }
}
