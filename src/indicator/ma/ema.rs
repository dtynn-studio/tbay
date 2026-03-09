use crate::{
    prelude::{Decimal, Indicator, KSummary},
    util::RingBuffer,
};

pub struct Ema {
    buffer: RingBuffer<Decimal>,
    current: Decimal,
    alpha: Decimal,
}

impl Ema {
    pub fn new(period: usize) -> Self {
        let alpha = Decimal::TWO / (Decimal::from(period) + Decimal::ONE);
        Self {
            buffer: RingBuffer::new(period),
            current: Decimal::ZERO,
            alpha,
        }
    }
}

impl Indicator for Ema {
    type Item = Decimal;
    type State = Decimal;

    fn state(&self) -> Option<&Self::State> {
        if self.buffer.is_full() {
            Some(&self.current)
        } else {
            None
        }
    }

    fn calc(&self, next: &KSummary) -> Option<Self::Item> {
        // 只有在buffer已满时才能计算EMA
        if !self.buffer.is_full() {
            return None;
        }

        let next = next.base.price_close;

        // 计算新的EMA值和差异
        let new_ema =
            self.alpha * next + (Decimal::ONE - self.alpha) * self.current;
        Some(new_ema)
    }

    fn update(&mut self, next: &KSummary) -> Option<Self::Item> {
        let next = next.base.price_close;

        // 1. 对buffer进行填充
        let removed = self.buffer.update(next);

        // 2. 在buffer不满时不做任何处理
        if self.buffer.is_full() {
            // 3. 在buffer第一次填满时，计算current值（简单平均）
            // 使用 buffer.is_full() && removed.is_none() 来判断是否是第一次填满
            if removed.is_none() {
                self.current = self.buffer.iter().sum::<Decimal>()
                    / Decimal::from(self.buffer.capacity());
            } else {
                // 4. 在之后的每次 update 中，使用EMA公式更新current值
                let new_ema = self.alpha * next
                    + (Decimal::ONE - self.alpha) * self.current;
                self.current = new_ema;
            }
            Some(self.current)
        } else {
            // buffer不满时不返回任何值
            None
        }
    }
}
