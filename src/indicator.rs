use std::{any::Any, cmp::Ordering, collections::HashMap, str::FromStr};

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

pub struct KCtx {
    pub info: KInfo,
    vals: HashMap<String, Box<dyn Any>>,
}

impl KCtx {
    pub fn get_val<T: 'static>(&self, key: &str) -> Option<&T> {
        self.vals.get(key).and_then(|o| o.downcast_ref())
    }

    pub fn set_val<T: 'static>(&mut self, key: &str, val: T) -> bool {
        self.vals.insert(key.to_owned(), Box::new(val)).is_some()
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

pub trait Indicator {
    type Output;

    fn key(&self) -> &str;
    fn deps(&self) -> Vec<&str>;
    fn calc(&self, next: &KCtx) -> Option<Self::Output>;
    fn update(&mut self, next: &KCtx) -> Option<Self::Output>;
}

pub trait IndicatorExt: Indicator + Sized + 'static {
    fn wrap_as_any(self) -> Box<dyn Indicator<Output = Box<dyn Any>>> {
        let inner = IndicatorAny(self);
        Box::new(inner)
    }
}

impl<I: Indicator + Sized + 'static> IndicatorExt for I {}

pub trait Calculator: FromStr<Err = Error> {
    fn calc(&self, next: Decimal) -> Option<Decimal>;
    fn update(&mut self, next: Decimal) -> Option<Decimal>;
}

pub struct IndicatorAny<I: Indicator>(I);

impl<I: Indicator> Indicator for IndicatorAny<I>
where
    I::Output: 'static,
{
    type Output = Box<dyn Any>;

    fn key(&self) -> &str {
        self.0.key()
    }

    fn deps(&self) -> Vec<&str> {
        self.0.deps()
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let raw = self.0.calc(next)?;
        Some(Box::new(raw))
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        let raw = self.0.update(next)?;
        Some(Box::new(raw))
    }
}
