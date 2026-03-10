use std::{cmp::Ordering, collections::HashMap, str::FromStr};

use rust_decimal::Decimal;
use time::OffsetDateTime;

use crate::prelude::Error;

pub mod base;
pub mod bollinger;
pub mod cross;
pub mod ma;
pub mod macd;
pub mod stddev;

pub const MA_IDX_PRICE: usize = 0;
pub const MA_IDX_QUNATITY: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RelativePosition {
    Above,
    Below,
    At,
}

#[derive(Clone, Copy)]
pub struct KRaw {
    pub time_begin: OffsetDateTime,
    pub time_end: OffsetDateTime,
    pub price_open: Decimal,
    pub price_close: Decimal,
    pub price_high: Decimal,
    pub price_low: Decimal,
    pub quantity: Decimal,
}

#[derive(Clone, Copy)]
pub struct PriceBar {
    pub high: Decimal,
    pub low: Decimal,
    pub mid: Decimal,
}

impl PriceBar {
    pub fn center_relative_position(&self, base: Decimal) -> RelativePosition {
        match self.mid.cmp(&base) {
            Ordering::Greater => RelativePosition::Above,
            Ordering::Less => RelativePosition::Below,
            Ordering::Equal => RelativePosition::At,
        }
    }

    pub fn full_relative_position(&self, base: Decimal) -> RelativePosition {
        if self.high <= base {
            RelativePosition::Below
        } else if self.low >= base {
            RelativePosition::Above
        } else {
            RelativePosition::At
        }
    }
}

#[derive(Clone, Copy)]
pub struct KInfo {
    pub raw: KRaw,
    pub body: PriceBar,
    pub full: PriceBar,
    pub quantity: Decimal,
}

impl KInfo {
    pub fn relative_position(&self, base: Decimal) -> RelativePosition {
        let body_rel_pos = self.body.center_relative_position(base);
        if body_rel_pos != RelativePosition::At {
            return body_rel_pos;
        }

        self.full.center_relative_position(base)
    }
}

#[derive(Clone)]
pub struct KSummary {
    pub info: KInfo,
    // 基础类指标，以键值对的形式储存，供更高级别的指标使用，如：
    // {
    //  "close:ema5": 1921.57,
    //  "close:sma20": 1901.45,
    //  "close:stddev20:": 1917.95,
    //  "qty:sam5:": 3156905.12,
    //  "qty:sam20:": 356905.12,
    // }
    pub bases: HashMap<String, Decimal>,
}

impl KSummary {
    pub fn get_base(&self, key: &str) -> Option<Decimal> {
        self.bases.get(key).cloned()
    }
}

// 基础类指标，以数值简单数值计算为主，如 ma、stddev 等
// 会在其他高级指标初始化时注册，以减少重复计算
pub trait BaseIndicator<'a>:
    Indicator<State = Decimal, Item<'a> = &'a KInfo, Value = Decimal>
    + FromStr<Err = Error>
{
    fn key(&self) -> &str;
}

pub trait Indicator: Sized {
    type State: Clone;
    type Item<'a>;
    type Value;

    fn state(&self) -> Option<&Self::State>;
    fn update<'a>(&mut self, next: Self::Item<'a>) -> Option<Self::Value>;
    fn calc<'a>(&self, next: Self::Item<'a>) -> Option<Self::Value>;
}
