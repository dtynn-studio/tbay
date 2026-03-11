use std::{cmp::Ordering, collections::HashMap, str::FromStr, sync::Arc};

use rust_decimal::Decimal;
use time::OffsetDateTime;

use crate::prelude::Error;

pub mod base;
pub mod bollinger;
pub mod cross;
pub mod distance;
pub mod ma;
pub mod macd;
pub mod position;
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

impl From<(Decimal, Decimal)> for PriceBar {
    fn from((p1, p2): (Decimal, Decimal)) -> Self {
        let (high, low) = if p1 > p2 { (p1, p2) } else { (p2, p1) };

        PriceBar::new(high, low)
    }
}

impl PriceBar {
    pub fn new(high: Decimal, low: Decimal) -> Self {
        Self {
            high,
            low,
            mid: (high + low) / Decimal::TWO,
        }
    }

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

    pub fn extremum(&self, position: bool) -> Decimal {
        if position { self.high } else { self.low }
    }
}

#[derive(Clone, Copy)]
pub struct KShadow {
    pub above: Decimal,
    pub below: Decimal,
}

#[derive(Clone, Copy)]
pub struct KInfo {
    pub raw: KRaw,
    pub direction: Option<bool>,
    pub body: PriceBar,
    pub full: PriceBar,
    pub shadow: KShadow,
}

impl From<KRaw> for KInfo {
    fn from(value: KRaw) -> Self {
        // body: 使用 open 和 close 计算
        let gte = value.price_close >= value.price_open;
        let (body, direction) = if gte {
            (
                PriceBar::new(value.price_close, value.price_open),
                if value.price_close == value.price_open {
                    None
                } else {
                    Some(gte)
                },
            )
        } else {
            (
                PriceBar::new(value.price_open, value.price_close),
                Some(false),
            )
        };

        // full: 使用 high 和 low 计算
        let full = PriceBar::new(value.price_high, value.price_low);

        // shadow: 影线
        let shadow = KShadow {
            above: full.high - body.high,
            below: body.low - full.low,
        };

        Self {
            raw: value,
            direction,
            body,
            full,
            shadow,
        }
    }
}

impl KInfo {
    pub fn relative_position(&self, base: Decimal) -> RelativePosition {
        let body_rel_pos = self.body.center_relative_position(base);
        if body_rel_pos != RelativePosition::At {
            return body_rel_pos;
        }

        self.full.center_relative_position(base)
    }

    pub fn is_not_below(&self, base: Decimal) -> bool {
        matches!(self.relative_position(base), RelativePosition::Below)
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
pub trait BaseIndicator:
    Indicator<State = Decimal, Item = Arc<KInfo>, Value = Decimal>
    + FromStr<Err = Error>
{
    fn key(&self) -> &str;
}

pub trait Indicator: Sized {
    type State: Clone;
    type Item;
    type Value;

    fn state(&self) -> Option<&Self::State>;
    fn update(&mut self, next: Self::Item) -> Option<Self::Value>;
    fn calc(&self, next: Self::Item) -> Option<Self::Value>;
}
