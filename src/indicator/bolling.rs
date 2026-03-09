use crate::prelude::{BaseKind, Decimal, Indicator, KSummary};

#[derive(Clone, Copy)]
pub struct BollingBandValue {
    pub mid: Decimal,
    pub up: Decimal,
    pub low: Decimal,
    pub dev: Decimal,
    pub bandwidth: Decimal,
}

pub struct BollingBand {
    mid_key: String,
    stddev_key: String,
    width: Decimal,
    current: Option<BollingBandValue>,
}

impl Indicator for BollingBand {
    type State = BollingBandValue;
    type Item = KSummary;
    type Value = BollingBandValue;

    fn state(&self) -> Option<&Self::State> {
        self.current.as_ref()
    }

    fn calc(&self, next: &Self::Item) -> Option<Self::Value> {
        let mid_opt = next.get_base(BaseKind::Price, &self.mid_key);
        let dev_opt = next.get_base(BaseKind::Price, &self.stddev_key);
        let (mid, dev) = mid_opt.zip(dev_opt)?;
        let bandwidth = dev * self.width;

        Some(BollingBandValue {
            mid,
            up: mid + bandwidth,
            low: mid - bandwidth,
            dev,
            bandwidth,
        })
    }

    fn update(&mut self, next: &Self::Item) -> Option<Self::Value> {
        let val = self.calc(next)?;
        self.current.replace(val);
        self.current
    }
}
