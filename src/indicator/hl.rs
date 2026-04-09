use std::borrow::Cow;

use crate::{
    impl_builder,
    indicator::{Builder, Indicator},
    prelude::{
        Args, Decimal, Error, FromStr, KCtx, ParseCtx, Result, ResultExt,
        Unexpected,
    },
    util::ring_buffer::RingBuffer,
};

pub struct Hl {
    key: String,
    buffer: RingBuffer<(Decimal, Decimal)>,
    current: Option<(Decimal, Decimal)>, // (low, high)
}

impl Hl {
    pub fn new(key: &str, period: usize) -> Self {
        Self {
            key: key.to_string(),
            buffer: RingBuffer::new(period),
            current: None,
        }
    }

    fn recalc(&self, start: usize) -> Option<(Decimal, Decimal)> {
        let (mut low, mut high) = self.buffer.get(start)?;
        let vals =
            (start + 1..self.buffer.size()).filter_map(|s| self.buffer.get(s));
        for (maybe_low, maybe_high) in vals {
            low = low.min(maybe_low);
            high = high.max(maybe_high);
        }

        Some((low, high))
    }
}

impl Indicator for Hl {
    type Output = (Decimal, Decimal); // (low, high)

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![]
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let high = next.info.raw.price_high;
        let low = next.info.raw.price_low;

        // 没有current值
        let Some((current_low, current_high)) = self.current else {
            return Some((low, high));
        };

        let maybe_replaced = if self.buffer.is_full() {
            self.buffer.get(0)
        } else {
            None
        };

        // 没有被代替的，即当前 buffer 不满，此时极值只由 (low, high) 和 self.current 决定
        let Some((replaced_low, replaced_high)) = maybe_replaced else {
            return Some((current_low.min(low), current_high.max(high)));
        };

        // 新值同时满足极大极小
        if high >= current_high && low <= current_low {
            return Some((low, high));
        }

        let low_may_changed = replaced_low == current_low;
        let high_may_changed = replaced_high == current_high;

        // 极值不会发生改变
        if !low_may_changed && !high_may_changed {
            return Some((current_low.min(low), current_high.max(high)));
        }

        // 极值可能发生变化
        self.recalc(1)
            .map(|(prev_low, prev_high)| {
                (prev_low.min(low), prev_high.max(high))
            })
            .or(Some((low, high)))
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        let next_high = next.info.raw.price_high;
        let next_low = next.info.raw.price_low;
        let replaced = self.buffer.update((next_high, next_low));

        let Some((current_low, current_high)) = self.current else {
            self.current.replace((next_low, next_high));
            return Some((next_low, next_high));
        };

        // 不发生替换，则 low/high 只会在 (replaced_low/low, replaced_high/high) 中产生
        let Some((replaced_low, replaced_high)) = replaced else {
            let low = current_low.min(next_low);
            let high = current_high.max(next_high);
            self.current.replace((low, high));
            return Some((low, high));
        };

        // 被替换的值有极值出现，则可能会有极值变换
        let maybe_changed =
            replaced_low == current_low || replaced_high == current_high;

        self.current = if maybe_changed {
            self.recalc(0)
        } else {
            Some((current_low.min(next_low), current_high.max(next_high)))
        };

        self.current
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HlArgs {
    period: usize,
}

impl FromStr for HlArgs {
    type Err = Error;

    // key format: hl:10
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut period = 0usize;

        scanf::sscanf!(s, "hl:{period}").with_context(|_| ParseCtx {
            raw: s.to_owned(),
            usage: Cow::from("parse Hl"),
        })?;

        if period == 0 {
            return Err(period.unexpected("hl period"));
        }

        Ok(Self { period })
    }
}

impl Args for HlArgs {
    type Type = usize;
    type Target = Hl;

    fn new(period: usize) -> Self {
        Self { period }
    }

    fn key(&self) -> String {
        format!("hl:{}", self.period)
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();
        Ok(Hl::new(&key, self.period))
    }
}

impl_builder!(HlBuilder: HlArgs => Hl);
