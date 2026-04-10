use std::{any::Any, cmp::Ordering, collections::HashMap};

use crate::{
    config::ColorTable,
    prelude::{Decimal, OffsetDateTime},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativePosition {
    Above,
    Below,
    At,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Up,
    Down,
    Unknown,
}

impl Trend {
    pub const fn as_str(self) -> &'static str {
        match self {
            Trend::Up => "↑",
            Trend::Down => "↓",
            Trend::Unknown => "~",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KRaw {
    pub time_begin: OffsetDateTime,
    pub time_end: OffsetDateTime,
    pub price_open: Decimal,
    pub price_close: Decimal,
    pub price_high: Decimal,
    pub price_low: Decimal,
    pub quantity: Decimal,
    pub trades: i64,
    pub finalized: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PriceBar {
    pub high: Decimal,
    pub low: Decimal,
    pub mid: Decimal,
    pub height: Decimal,
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
            height: high - low,
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

#[derive(Debug, Clone, Copy)]
pub struct KShadow {
    pub above: Decimal,
    pub below: Decimal,
}

#[derive(Debug, Clone, Copy)]
pub struct KInfo {
    pub raw: KRaw,
    pub direction: Option<bool>,
    pub body: PriceBar,
    pub full: PriceBar,
    pub shadow: KShadow,
    pub trend: Trend,
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

        let trend = kinfo_trend(body, full, shadow, direction, None);

        Self {
            raw: value,
            direction,
            body,
            full,
            shadow,
            trend,
        }
    }
}

fn kinfo_trend(
    body: PriceBar,
    full: PriceBar,
    shadow: KShadow,
    direction: Option<bool>,
    threshold: Option<Decimal>,
) -> Trend {
    let threshold = threshold.unwrap_or(Decimal::from(2) / Decimal::from(3));

    if full.height.is_zero() {
        return Trend::Unknown;
    }

    let body_height = body.high - body.low;

    if body_height / full.height >= threshold {
        match direction {
            Some(true) => Trend::Up,
            Some(false) => Trend::Down,
            None => Trend::Unknown,
        }
    } else if shadow.above / full.height >= threshold {
        Trend::Down
    } else if shadow.below / full.height >= threshold {
        Trend::Up
    } else {
        Trend::Unknown
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
        !matches!(self.relative_position(base), RelativePosition::Below)
    }
}

pub struct KCtx {
    pub info: KInfo,
    pub colors: ColorTable,
    vals: HashMap<String, Box<dyn Any>>,
}

// impl From<KRaw> for KCtx {
//     fn from(value: KRaw) -> Self {
//         KCtx {
//             info: value.into(),
//             vals: Default::default(),
//         }
//     }
// }

impl KCtx {
    pub fn new(raw: KRaw, colors: ColorTable) -> Self {
        Self {
            info: raw.into(),
            colors,
            vals: Default::default(),
        }
    }

    pub fn get_val<T: 'static>(&self, key: &str) -> Option<&T> {
        self.vals.get(key).and_then(|o| o.downcast_ref())
    }

    pub fn set_val(&mut self, key: &str, val: Box<dyn Any>) -> bool {
        self.vals.insert(key.to_owned(), val).is_some()
    }
}
