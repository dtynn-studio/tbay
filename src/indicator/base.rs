use std::str::FromStr;

use crate::{
    indicator::{
        Calculator,
        ma::{Ema, Sma},
        stddev::StdDev,
    },
    prelude::{Decimal, Error},
    res::Unexpected,
};

mod extract;

pub use extract::{BaseExtractor, BaseExtractorBuilder, CalcKind, ExtractKind};

pub enum BaseCalculator {
    Sma(Sma),
    Ema(Ema),
    StdDev(StdDev),
}

impl FromStr for BaseCalculator {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse()
            .map(BaseCalculator::Sma)
            .or_else(|_| s.parse().map(BaseCalculator::Ema))
            .or_else(|_| s.parse().map(BaseCalculator::StdDev))
            .map_err(|_| s.unexpected("base calculator"))
    }
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
