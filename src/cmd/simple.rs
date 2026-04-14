use clap::Parser;
use humantime::Duration;
use tracing::info;

use crate::{
    event::binance::client::{BnClient, Config, WebsocketControl},
    prelude::*,
};

#[derive(Parser)]
pub struct SimpleArgs {
    #[arg(long)]
    pub pair: String,

    #[arg(long)]
    pub interval: Duration,

    #[arg(from_global)]
    pub testnet: bool,

    #[arg(from_global)]
    pub proxy: Option<String>,
}

impl SimpleArgs {
    pub async fn run(self) -> Result<()> {
        info!(pair = self.pair, interval = %self.interval, "simple runs");

        let target = Target {
            symbol: self.pair,
            interval: self.interval.into(),
        };

        let client = BnClient::new(Config {
            testnet: self.testnet,
            proxy: self.proxy,
        })?;

        let (mut event_rx, ctrl_tx) =
            client.subscribe_klines(&[target]).await?;

        ctrlc::set_handler(move || {
            info!("stop signal captured");
            _ = ctrl_tx.send(WebsocketControl::Disconnect);
        })
        .context(SignalCtx)?;

        while let Some(evt) = event_rx.recv().await {
            match evt {
                Event::K(k) => {
                    info!(?k, "k received");
                }

                Event::Disconnect(reason) => {
                    info!(reason, "disconnect");
                    break;
                }

                Event::Broken(reason) => {
                    info!(reason, "broken");
                    break;
                }
            }
        }

        info!("stopped");
        Ok(())
    }
}
