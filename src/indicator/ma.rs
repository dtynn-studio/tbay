mod ema;
mod sma;

pub use ema::Ema;
pub use sma::Sma;

use crate::prelude::{Decimal, Indicator};

pub enum Ma {
    Sma(Sma),
    Ema(Ema),
}

impl Indicator for Ma {
    type State = Decimal;
    type Item = Decimal;
    type Value = Decimal;

    fn key(&self) -> &str {
        match self {
            Self::Sma(m) => m.key(),
            Self::Ema(m) => m.key(),
        }
    }

    fn state(&self) -> Option<&Self::State> {
        match self {
            Self::Sma(m) => m.state(),
            Self::Ema(m) => m.state(),
        }
    }

    fn calc(&self, next: Self::Item) -> Option<Self::Value> {
        match self {
            Self::Sma(m) => m.calc(next),
            Self::Ema(m) => m.calc(next),
        }
    }

    fn update(&mut self, next: Self::Item) -> Option<Self::Value> {
        match self {
            Self::Sma(m) => m.update(next),
            Self::Ema(m) => m.update(next),
        }
    }

    fn deps(&self) -> Vec<String> {
        match self {
            Self::Sma(m) => m.deps(),
            Self::Ema(m) => m.deps(),
        }
    }
}
