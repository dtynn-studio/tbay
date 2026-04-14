use crate::{
    indicator::{
        Calculator,
        ma::{Ema, Sma},
        stddev::StdDev,
    },
    prelude::Decimal,
};

mod extract;

pub use extract::{
    BaseExtractor, BaseExtractorArgs, BaseExtractorBuilder, CalcKind,
    ExtractKind, extractor,
};

pub enum BaseCalculator {
    Sma(Sma),
    Ema(Ema),
    StdDev(StdDev),
}

impl Calculator for BaseCalculator {
    fn calc(&self, next: Decimal) -> Option<Decimal> {
        match self {
            Self::Sma(m) => m.calc(next),
            Self::Ema(m) => m.calc(next),
            Self::StdDev(m) => m.calc(next),
        }
    }

    fn update(&mut self, next: Decimal) -> Option<Decimal> {
        match self {
            Self::Sma(m) => m.update(next),
            Self::Ema(m) => m.update(next),
            Self::StdDev(m) => m.update(next),
        }
    }
}
