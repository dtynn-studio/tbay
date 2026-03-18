use crossbeam_channel::Sender;

use crate::prelude::{KRaw, Result};

pub mod binance;

#[derive(Debug)]
pub struct K {
    pub symbol: String,
    pub interval: String,
    pub raw: KRaw,
}

#[derive(Debug)]
pub enum Event {
    K(K),
}

pub type EventChanTx = Sender<Event>;

pub struct Target {
    pub symbol: String,
    pub interval: String,
}

pub trait DataSource {
    fn new(event_tx: EventChanTx) -> Self;

    fn start(self, targets: Vec<Target>) -> Result<impl SubscribeStopper>;
}

pub trait SubscribeStopper {
    fn stop(self);
}
