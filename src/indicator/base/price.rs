use std::{borrow::Cow, str::FromStr};

use scanf::sscanf;
use snafu::ResultExt;

use crate::{
    indicator::ma::{Ema, Ma, Sma},
    prelude::{
        BaseIndicator, Decimal, Error, Indicator, KInfo, ParseCtx, Unexpected,
    },
};

fn close_extractor(info: &KInfo) -> Decimal {
    info.raw.price_close
}

pub struct PriceMa {
    key: String,
    ma: Ma,
    extractor: fn(&KInfo) -> Decimal,
}

impl FromStr for PriceMa {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut period: usize = 0;
        let mut price_kind: String = String::new();
        let mut ma_kind: String = String::new();

        sscanf!(s, "{price_kind}:{ma_kind}:{period}").with_context(|_| {
            ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse PriceMa"),
            }
        })?;

        let extractor = match price_kind.as_str() {
            "close" => close_extractor,
            other => return Err(other.unexpected("price kind")),
        };

        if period == 0 {
            return Err(period.unexpected("ma period"));
        }

        let ma = match ma_kind.as_str() {
            "ema" => Ma::Ema(Ema::new(period)),
            "sma" => Ma::Sma(Sma::new(period)),
            other => return Err(other.unexpected("ma kind")),
        };

        Ok(Self {
            key: s.to_owned(),
            ma,
            extractor,
        })
    }
}

impl Indicator for PriceMa {
    type State = Decimal;
    type Item<'a> = &'a KInfo;
    type Value = Decimal;

    fn state(&self) -> Option<&Self::State> {
        self.ma.state()
    }

    fn calc<'a>(&self, next: Self::Item<'a>) -> Option<Self::Value> {
        let next = (self.extractor)(next);
        self.ma.calc(next)
    }

    fn update<'a>(&mut self, next: Self::Item<'a>) -> Option<Self::Value> {
        let next = (self.extractor)(next);
        self.ma.update(next)
    }
}

impl<'a> BaseIndicator<'a> for PriceMa {
    fn key(&self) -> &str {
        &self.key
    }
}
