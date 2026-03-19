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

        hub.apply_config(cfg)?;

        if self.dry {
            info!("dry run, stopped");
            return Ok(());
        }

        Ok(())
    }
}
