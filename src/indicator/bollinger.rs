use crate::prelude::{Arc, Decimal, Indicator, KSummary};

#[derive(Clone, Copy)]
pub struct BollingerBandValue {
    pub mid: Decimal,
    pub up: Decimal,
    pub low: Decimal,
    pub dev: Decimal,
    pub bandwidth: Decimal,
}

pub struct BollingerBand {
    mid_key: String,
    stddev_key: String,
    width: Decimal,
    current: Option<BollingerBandValue>,
}

impl Indicator for BollingerBand {
    type State = BollingerBandValue;
    type Item = Arc<KSummary>;
    type Value = BollingerBandValue;

    fn state(&self) -> Option<&Self::State> {
        self.current.as_ref()
    }

    fn calc(&self, next: Self::Item) -> Option<Self::Value> {
        let next = next.as_ref();
        let mid_opt = next.get_base(&self.mid_key);
        let dev_opt = next.get_base(&self.stddev_key);
        let (mid, dev) = mid_opt.zip(dev_opt)?;
        let bandwidth = dev * self.width;

        Some(BollingerBandValue {
            mid,
            up: mid + bandwidth,
            low: mid - bandwidth,
            dev,
            bandwidth,
        })
    }

    fn update(&mut self, next: Self::Item) -> Option<Self::Value> {
        let val = self.calc(next)?;
        self.current.replace(val);
        self.current
    }
}
