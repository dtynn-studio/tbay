use std::{borrow::Cow, cmp::Ordering};

use crate::{
    indicator::{
        Builder,
        base::{BaseExtractorArgs, CalcKind, ExtractKind},
    },
    prelude::{
        Args, Decimal, Error, FromStr, Indicator, KCtx, ParseCtx, Result,
        Unexpected,
    },
};
use snafu::ResultExt;

#[derive(Clone)]
pub struct CrossItem<T> {
    pub fast: T,
    pub slow: T,
    pub pos: Ordering,
}

impl<T: Ord + Clone> CrossItem<T> {
    pub fn new(fast: T, slow: T) -> Self {
        let pos = fast.cmp(&slow);
        Self { fast, slow, pos }
    }
}

#[derive(Clone)]
pub struct CrossValue<T: Clone> {
    pub prev: CrossItem<T>,
    pub next: CrossItem<T>,
    // 上穿为 Some(true),
    // 下穿为 Some(false)
    // 未发生穿越则为 None
    pub cross: Option<bool>,
}

fn calc_cross<T: Ord>(
    prev: &CrossItem<T>,
    next: &CrossItem<T>,
) -> Option<bool> {
    match (prev.pos, next.pos) {
        // 在同侧
        (Ordering::Greater, Ordering::Greater)
        | (Ordering::Less, Ordering::Less) => None,

        (Ordering::Equal, Ordering::Equal) => match next.fast.cmp(&prev.fast) {
            Ordering::Greater => Some(true),
            Ordering::Less => Some(false),
            Ordering::Equal => None,
        },

        (Ordering::Less, Ordering::Equal)
        | (Ordering::Less, Ordering::Greater)
        | (Ordering::Equal, Ordering::Greater) => Some(true),

        (Ordering::Greater, Ordering::Equal)
        | (Ordering::Greater, Ordering::Less)
        | (Ordering::Equal, Ordering::Less) => Some(false),
    }
}

#[derive(Clone, Default)]
pub struct Cross<T: Clone> {
    prev: Option<CrossItem<T>>,
}

impl<T: Ord + Clone + 'static> Cross<T> {
    pub fn calc(&self, next: CrossItem<T>) -> Option<CrossValue<T>> {
        let prev = self.prev.as_ref()?;

        let cross = calc_cross(prev, &next);

        Some(CrossValue {
            prev: prev.clone(),
            next,
            cross,
        })
    }

    pub fn update(&mut self, next: CrossItem<T>) -> Option<CrossValue<T>> {
        let prev = match self.prev.take() {
            Some(p) => {
                self.prev.replace(next.clone());
                p
            }

            None => {
                self.prev.replace(next);
                return None;
            }
        };

        let cross = calc_cross(&prev, &next);
        let value = CrossValue { prev, next, cross };

        Some(value)
    }
}

pub struct MaCross {
    key: String,
    fast_key: String,
    slow_key: String,
    current: Cross<Decimal>,
}

fn build_cross_item(
    next: &KCtx,
    fast_key: &str,
    slow_key: &str,
) -> Option<CrossItem<Decimal>> {
    let fast = next.get_val(fast_key).copied()?;
    let slow = next.get_val(slow_key).copied()?;
    Some(CrossItem::new(fast, slow))
}

impl Indicator for MaCross {
    type Output = CrossValue<Decimal>;

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.fast_key, &self.slow_key]
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let item = build_cross_item(next, &self.fast_key, &self.slow_key)?;
        self.current.calc(item)
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        let item = build_cross_item(next, &self.fast_key, &self.slow_key)?;
        self.current.update(item)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MaCrossArgs {
    pub val_kind: ExtractKind,
    pub calc_kind: CalcKind,
    pub fast: usize,
    pub slow: usize,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct MaCrossBuilder;

impl Builder for MaCrossBuilder {
    type Target = MaCross;

    fn build(&self, s: &str) -> Result<Self::Target> {
        let args = MaCrossArgs::from_str(s)?;
        args.build()
    }
}

impl FromStr for MaCrossArgs {
    type Err = Error;

    // format: cross:close,ema,5,20
    // format: cross:qty,sma,20,60
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut val_kind_str = String::new();
        let mut calc_kind_str = String::new();
        let mut fast = 0usize;
        let mut slow = 0usize;

        scanf::sscanf!(s, "cross:{val_kind_str},{calc_kind_str},{fast},{slow}")
            .with_context(|_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse MaCross"),
            })?;

        let val_kind = val_kind_str.parse()?;

        let calc_kind = calc_kind_str.parse()?;

        if fast == 0 {
            return Err(fast.unexpected("ma cross fast period"));
        }

        if slow == 0 {
            return Err(slow.unexpected("ma cross slow period"));
        }

        Ok(Self {
            val_kind,
            calc_kind,
            fast,
            slow,
        })
    }
}

impl Args for MaCrossArgs {
    type Type = (ExtractKind, CalcKind, usize, usize);
    type Target = MaCross;

    fn new(args: Self::Type) -> Self {
        Self {
            val_kind: args.0,
            calc_kind: args.1,
            fast: args.2,
            slow: args.3,
        }
    }

    fn key(&self) -> String {
        format!(
            "cross:{},{},{},{}",
            self.val_kind.as_str(),
            self.calc_kind.as_str(),
            self.fast,
            self.slow
        )
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();

        let fast_key =
            BaseExtractorArgs::new((self.val_kind, self.calc_kind, self.fast))
                .key();

        let slow_key =
            BaseExtractorArgs::new((self.val_kind, self.calc_kind, self.slow))
                .key();

        Ok(MaCross {
            key,
            fast_key,
            slow_key,
            current: Cross::default(),
        })
    }
}
