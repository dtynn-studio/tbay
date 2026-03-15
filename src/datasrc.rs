use crate::prelude::{KRaw, Result};

pub struct K {
    pub symbol: String,
    pub interval: String,
    pub raw: KRaw,
}

pub struct Target {
    pub symbol: String,
    pub interval: String,
}

pub trait DataSource {
    fn subscribe(&self, targets: &[Target]) -> Result<impl Iterator<Item = K>>;
}
