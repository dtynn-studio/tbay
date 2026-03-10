use std::str::FromStr;

use crate::{
    indicator::ma::Ma,
    prelude::{BaseIndicator, Decimal, Error, Indicator, KInfo},
};

fn close_extractor(info: &KInfo) -> Decimal {
    info.raw.price_close
}

pub struct PriceMa {
    ma: Ma,
    extractor: fn(&KInfo) -> Decimal,
}

impl FromStr for PriceMa {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        unimplemented!()
    }
}

impl Indicator for PriceMa {
    type State = Decimal;
    type Item<'a> = &'a KInfo;
    type Value = Decimal;

    fn state(&self) -> Option<&Self::State> {
        unimplemented!()
    }

    fn calc<'a>(&self, ext: Self::Item<'a>) -> Option<Self::Value> {
        unimplemented!()
    }

    fn update<'a>(&mut self, next: Self::Item<'a>) -> Option<Self::Value> {
        unimplemented!()
    }
}

impl<'a> BaseIndicator<'a> for PriceMa {
    fn key(&self) -> &str {
        unimplemented!()
    }
}
