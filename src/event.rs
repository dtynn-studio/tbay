use crossbeam_channel::Sender;

use crate::prelude::{KRaw, Result};

pub mod binance;

pub struct K {
    pub symbol: String,
    pub interval: String,
    pub raw: KRaw,
}

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

    fn subscribe(
        &mut self,
        targets: &[Target],
    ) -> Result<impl SubscribeStopper>;
}

pub trait SubscribeStopper {
    fn stop(self);
}
