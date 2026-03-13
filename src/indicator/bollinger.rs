use crate::{
    indicator::{
        Indicator,
        base::{BaseExtractorBuilder, CalcKind, ExtractKind},
    },
    prelude::{Builder, Decimal, FromPrimitive, KCtx, Result},
    res::Unexpected,
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

impl BollingerBand {
    pub fn new(period: usize, width: usize) -> Result<Self> {
        let key = format!("bb:{period},{width}");
        let mid_key = BaseExtractorBuilder::new((
            ExtractKind::PriceClose,
            CalcKind::Ema,
            period,
        ))
        .key();

        let stddev_key = BaseExtractorBuilder::new((
            ExtractKind::PriceClose,
            CalcKind::StdDev,
            period,
        ))
        .key();

        let width = Decimal::from_usize(width)
            .ok_or_else(|| width.unexpected("bollinger band width"))?;

        Ok(Self {
            key,
            mid_key,
            stddev_key,
            width,
            current: None,
        })
    }
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
