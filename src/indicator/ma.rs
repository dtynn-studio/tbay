mod ema;
mod sma;

pub use ema::Ema;
pub use sma::Sma;

use std::str::FromStr;

use crate::{
    indicator::Calculator,
    prelude::{Decimal, Error},
};

pub enum Ma {
    Sma(Sma),
    Ema(Ema),
}

impl FromStr for Ma {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(sma) = s.parse().map(Ma::Sma) {
            Ok(sma)
        } else if let Ok(ema) = s.parse().map(Ma::Ema) {
            Ok(ema)
        } else {
            Err(Error::Msg {
                reason: format!("invalid ma: {}", s).into(),
            })
        }
    }
}

impl Calculator for Ma {
    fn calc(&self, next: Decimal) -> Option<Decimal> {
        match self {
            Self::Sma(m) => m.calc(next),
            Self::Ema(m) => m.calc(next),
        }
    }

    fn update(&mut self, next: Decimal) -> Option<Decimal> {
        match self {
            Self::Sma(m) => m.update(next),
            Self::Ema(m) => m.update(next),
        }
    }
}
