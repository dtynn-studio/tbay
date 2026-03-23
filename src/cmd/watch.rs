use std::path::PathBuf;

use clap::Parser;
use tracing::{error, info};

use crate::{
    config::load_config,
    event::binance::client::{BnClient, Config},
    hub::Hub,
    prelude::*,
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
}

impl WatchArgs {
    pub async fn run(self) -> Result<()> {
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

        for target in targets {
            match client.load_k_history(&target).await {
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
