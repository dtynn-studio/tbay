use crate::{
    indicator::Indicator,
    prelude::{Decimal, KCtx},
};

#[derive(Clone, Copy)]
pub struct BollingerBandValue {
    pub mid: Decimal,
    pub up: Decimal,
    pub low: Decimal,
    pub dev: Decimal,
    pub bandwidth: Decimal,
}

pub struct BollingerBand {
    key: String,
    mid_key: String,
    stddev_key: String,
    width: Decimal,
    current: Option<BollingerBandValue>,
}

impl Indicator for BollingerBand {
    type Output = BollingerBandValue;

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.mid_key, &self.stddev_key]
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let mid = *next.get_val::<Decimal>(&self.mid_key)?;
        let dev = *next.get_val::<Decimal>(&self.stddev_key)?;
        let bandwidth = dev * self.width;

        Some(BollingerBandValue {
            mid,
            up: mid + bandwidth,
            low: mid - bandwidth,
            dev,
            bandwidth,
        })
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        let value = self.calc(next)?;
        self.current.replace(value);
        Some(value)
    }
}
