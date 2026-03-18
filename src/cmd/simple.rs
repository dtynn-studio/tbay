use clap::Parser;
use crossbeam_channel::unbounded;
use tracing::info;

use crate::{event::binance::fut::BinanceDataSource, prelude::*};

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

        let target = Target {
            symbol: self.pair,
            interval: self.interval,
        };

        let (tx, rx) = unbounded();

        let bn_src = BinanceDataSource::new(tx);
        let targets = vec![target];
        let stopper = bn_src.start(targets)?;

        let mut stopper_opt = Some(stopper);
        ctrlc::set_handler(move || {
            info!("stop signal captured");
            if let Some(stopper) = stopper_opt.take() {
                stopper.stop();
            }
        })
        .context(SignalCtx)?;

        while let Ok(evt) = rx.recv() {
            info!(?evt, "received");
        }

        info!("stopped");
        Ok(())
    }
}
