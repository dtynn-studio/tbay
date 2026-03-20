use std::path::PathBuf;

use clap::Parser;
use tracing::{error, info};

use crate::{
    config::load_config, event::binance::fut::FutClient, hub::Hub, prelude::*,
};

#[derive(Parser)]
pub struct WatchArgs {
    #[arg(long, default_value_t = false)]
    pub dry: bool,

    #[arg(long)]
    pub config: PathBuf,

    #[arg(long, default_value_t = 1000)]
    pub history: usize,
}

impl WatchArgs {
    pub fn run(self) -> Result<()> {
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

        let client = FutClient::new(false, false)?;
        for target in targets {
            match client.load_history(&target, self.history) {
                Ok(ks) => {
                    for k in ks {
                        hub.apply_k(k);
                    }
                }
                Err(e) => {
                    error!(t = ?target, "load history: {e:?}");
                }
            }
        }

        info!("history ks loaded");

        let states = hub.states();

        for hs in states {
            println!("Hub State for {}", hs.symbol);
            for (d, sts) in hs.states {
                println!("\t{d}");
                for st in sts {
                    println!("\t\ttemp: {:?}, perm: {:?}", st.temp, st.perm);
                }
            }
        }

        Ok(())
    }
}
