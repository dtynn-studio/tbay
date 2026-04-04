use crossbeam_channel::Sender;
use humantime::Duration;

use crate::prelude::{KRaw, Result};

pub mod binance;

#[derive(Debug, Clone)]
pub struct K {
    pub symbol: String,
    pub interval: Duration,
    pub source: &'static str,
    pub raw: KRaw,
}

#[derive(Debug, Clone)]
pub enum Event {
    K(K),
    Disconnect(String),
    Broken(String),
}

pub type EventChanTx = Sender<Event>;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Target {
    pub symbol: String,
    pub interval: Duration,
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.symbol, self.interval)
    }
}

impl Target {
    pub fn bn_futures_perpetual_key(&self) -> String {
        format!(
            "{}_perpetual@continuousKline_{}",
            self.symbol, self.interval
        )
    }

    pub fn bn_futures_key(&self) -> String {
        format!("{}@kline_{}", self.symbol, self.interval)
    }
}

pub trait DataSource {
    fn new(event_tx: EventChanTx) -> Self;

    fn start(self, targets: Vec<Target>) -> Result<impl SubscribeStopper>;
}

pub trait SubscribeStopper {
    fn stop(self);
}
