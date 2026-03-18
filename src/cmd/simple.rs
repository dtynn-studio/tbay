use clap::Parser;
use tracing::info;

use crate::prelude::*;

#[derive(Parser)]
pub struct SimpleArgs {
    #[arg(long)]
    pub pair: String,

    #[arg(long)]
    pub interval: String,
}

impl SimpleArgs {
    pub fn run(self) -> Result<()> {
        info!(pair = self.pair, interval = self.interval, "simple runs");
        Ok(())
    }
}
