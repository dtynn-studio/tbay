use std::{borrow::Cow, str::FromStr, sync::Arc};

use scanf::sscanf;
use snafu::ResultExt;

use crate::{
    indicator::ma::Ma,
    prelude::{
        BaseIndicator, Decimal, Error, Indicator, KInfo, ParseCtx, Unexpected,
    },
};

fn close_extractor(info: &KInfo) -> Decimal {
    info.raw.price_close
}

fn qty_extractor(info: &KInfo) -> Decimal {
    info.raw.quantity
}

pub struct BaseExtractMa {
    key: String,
    ma: Ma,
    extractor: fn(&KInfo) -> Decimal,
}

impl FromStr for BaseExtractMa {
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

        let ma = if let Ok(sma) = sub.parse().map(Ma::Sma) {
            sma
        } else if let Ok(ema) = sub.parse().map(Ma::Ema) {
            ema
        } else {
            return Err(sub.unexpected("ma kind"));
        };

        Ok(Self {
            key: s.to_owned(),
            ma,
            extractor,
        })
    }
}

impl Indicator for BaseExtractMa {
    type State = Decimal;
    type Item = Arc<KInfo>;
    type Value = Decimal;

    fn key(&self) -> &str {
        &self.key
    }

    fn state(&self) -> Option<&Self::State> {
        self.ma.state()
    }

    fn calc(&self, next: Self::Item) -> Option<Self::Value> {
        let next = (self.extractor)(next.as_ref());
        self.ma.calc(next)
    }

    fn update(&mut self, next: Self::Item) -> Option<Self::Value> {
        let next = (self.extractor)(next.as_ref());
        self.ma.update(next)
    }

    fn deps(&self) -> Vec<String> {
        vec![]
    }
}

impl BaseIndicator for BaseExtractMa {
    fn key(&self) -> &str {
        &self.key
    }
}
