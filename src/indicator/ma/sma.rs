use std::{borrow::Cow, str::FromStr};

use scanf::sscanf;
use snafu::ResultExt;

use crate::{
    indicator::Calculator,
    prelude::{Decimal, Error},
    res::{ParseCtx, Unexpected},
    util::RingBuffer,
};

pub struct Sma {
    buffer: RingBuffer<Decimal>,
    current: Decimal,
    period: Decimal,
}

impl FromStr for Sma {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut kind = String::new();
        let mut period = 0usize;

        sscanf!(s, "{kind}:{period}").with_context(|_| ParseCtx {
            raw: s.to_owned(),
            usage: Cow::from("parse Sma"),
        })?;

        if kind != "sma" {
            return Err(kind.unexpected("sma kind"));
        }

        if period == 0 {
            return Err(period.unexpected("sma period"));
        }

        Ok(Self::new(period))
    }
}

impl Sma {
    pub fn new(period: usize) -> Self {
        Self {
            buffer: RingBuffer::new(period),
            current: Decimal::ZERO,
            period: Decimal::from(period),
        }
    }
}

impl Calculator for Sma {
    fn calc(&self, next: Decimal) -> Option<Decimal> {
        // 只有在buffer已满时才能计算SMA
        if !self.buffer.is_full() {
            return None;
        }

        // 获取即将被替换的值（最老的值）
        // 在满的RingBuffer中，最老的元素是索引为0的元素，且必定存在
        let removed_value = self.buffer.get(0)?;

        // 计算新的SMA值和差异
        let diff = (next - removed_value) / self.period;
        let new_sma = self.current + diff;
        Some(new_sma)
    }

    fn update(&mut self, next: Decimal) -> Option<Decimal> {
        // 1. 对buffer进行填充
        let removed = self.buffer.update(next);

        // 2. 在buffer不满时不做任何处理
        if self.buffer.is_full() {
            if let Some(removed_value) = removed {
                // 4. 在之后的每次 update 中，仅更新 removed 和 next 带来的值变化
                let diff = (next - removed_value) / self.period;
                self.current += diff;
            } else {
                // 3. 在buffer第一次填满时，计算current值
                self.current =
                    self.buffer.iter().sum::<Decimal>() / self.period;
            }

            Some(self.current)
        } else {
            // buffer不满时不返回任何值
            None
        }
    }
}
