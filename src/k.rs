use std::{any::Any, cmp::Ordering, collections::HashMap};

use crate::{
    config::ColorTable,
    indicator::base::ExtractKind,
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
        !matches!(self.relative_position(base), RelativePosition::Below)
    }

    pub fn trend(
        &self,
        single_threshold: Decimal,
        mixed_threshold: Decimal,
    ) -> Trend {
        if self.full.height.is_zero() {
            return Trend::Unknown;
        }

        let body_ratio = self.body.height / self.full.height;
        let above_ratio = self.shadow.above / self.full.height;
        let below_ratio = self.shadow.below / self.full.height;

        if body_ratio >= single_threshold {
            match self.direction {
                Some(true) => Trend::Up,
                Some(false) => Trend::Down,
                None => Trend::Unknown,
            }
        } else if above_ratio >= single_threshold {
            Trend::Down
        } else if below_ratio >= single_threshold {
            Trend::Up
        } else {
            // 没有单个部分满足占比
            match self.direction {
                Some(true) => {
                    if (below_ratio + body_ratio) >= mixed_threshold {
                        Trend::Up
                    } else {
                        Trend::Unknown
                    }
                }

                Some(false) => {
                    if (above_ratio + body_ratio) >= mixed_threshold {
                        Trend::Down
                    } else {
                        Trend::Unknown
                    }
                }

                None => Trend::Unknown,
            }
        }
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

#[derive(Debug)]
pub struct StrengthChecker {
    val_kind: ExtractKind,
    pub key: String,
    thres: Decimal,
}

impl StrengthChecker {
    pub fn new(val_kind: ExtractKind, key: String, thres: Decimal) -> Self {
        Self {
            val_kind,
            key,
            thres,
        }
    }

    pub fn check(&self, kctx: &KCtx) -> bool {
        let Some(ma) = kctx.get_val::<Decimal>(&self.key) else {
            return false;
        };

        if ma.is_zero() {
            return false;
        }

        let next = self.val_kind.extractor()(&kctx.info);
        (next / ma) > self.thres
    }
}
