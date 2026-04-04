use std::borrow::Cow;

use crate::{
    impl_builder,
    indicator::{
        Builder, Indicator,
        base::{BaseExtractorArgs, CalcKind, ExtractKind},
    },
    prelude::{
        Args, Decimal, Error, FromPrimitive, FromStr, KCtx, ParseCtx, Result,
        ResultExt, Unexpected,
    },
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

#[derive(Clone, Copy)]
pub struct BollingerBandArgs {
    period: usize,
    width: usize,
}

impl FromStr for BollingerBandArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut period = 0usize;
        let mut width = 0usize;

        scanf::sscanf!(s, "bb:{period},{width}").with_context(|_| {
            ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse BollingerBand"),
            }
        })?;

        if period == 0 {
            return Err(period.unexpected("bollinger band period"));
        }

        if width == 0 {
            return Err(width.unexpected("bollinger band width"));
        }

        Ok(Self { period, width })
    }
}

impl Args for BollingerBandArgs {
    type Type = (usize, usize);
    type Target = BollingerBand;

    fn new(args: Self::Type) -> Self {
        Self {
            period: args.0,
            width: args.1,
        }
    }

    fn key(&self) -> String {
        format!("bb:{},{}", self.period, self.width)
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();

        let mid_key = BaseExtractorArgs::new((
            ExtractKind::PriceClose,
            CalcKind::Ema,
            self.period,
        ))
        .key();

        let stddev_key = BaseExtractorArgs::new((
            ExtractKind::PriceClose,
            CalcKind::StdDev,
            self.period,
        ))
        .key();

        let width = Decimal::from_usize(self.width)
            .ok_or_else(|| self.width.unexpected("bollinger band width"))?;

        Ok(BollingerBand {
            key,
            mid_key,
            stddev_key,
            width,
            current: None,
        })
    }
}

impl_builder!(BollingerBandBuilder: BollingerBandArgs => BollingerBand);

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
