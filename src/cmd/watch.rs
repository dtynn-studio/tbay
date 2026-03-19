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

        Ok(())
    }
}
