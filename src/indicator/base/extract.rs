use std::{borrow::Cow, str::FromStr};

use scanf::sscanf;
use snafu::ResultExt;

use crate::{
    indicator::{Calculator, Indicator, base::BaseCalculator},
    prelude::{Decimal, Error, KCtx, KInfo, ParseCtx, Unexpected},
};

fn close_extractor(info: &KInfo) -> Decimal {
    info.raw.price_close
}

fn qty_extractor(info: &KInfo) -> Decimal {
    info.raw.quantity
}

pub struct BaseExtractor {
    key: String,
    calculator: BaseCalculator,
    extractor: fn(&KInfo) -> Decimal,
}

impl FromStr for BaseExtractor {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut extract_kind: String = String::new();
        let mut sub: String = String::new();

        sscanf!(s, "{extract_kind}:{sub}").with_context(|_| ParseCtx {
            raw: s.to_owned(),
            usage: Cow::from("parse PriceMa"),
        })?;

        let extractor = match extract_kind.as_str() {
            "close" => close_extractor,
            "qty" => qty_extractor,
            other => return Err(other.unexpected("extract kind")),
        };

        let calculator: BaseCalculator =
            sub.parse().map_err(|_| sub.unexpected("calculator kind"))?;

        Ok(Self {
            key: s.to_owned(),
            calculator,
            extractor,
        })
    }
}

impl Indicator for BaseExtractor {
    type Output = Decimal;

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![]
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let value = (self.extractor)(&next.info);
        self.calculator.calc(value)
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        let value = (self.extractor)(&next.info);
        self.calculator.update(value)
    }
}
