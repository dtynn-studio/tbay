use rust_decimal::MathematicalOps;

use crate::{
    prelude::{Decimal, Indicator},
    util::RingBuffer,
};

/// 根据给定的总和、平方和和周期计算标准差
fn compute_std_dev(
    sum: Decimal,
    sum_squares: Decimal,
    period: Decimal,
) -> Decimal {
    let mean = sum / period;
    let variance = (sum_squares / period) - (mean * mean);

    // 防止由于浮点精度问题导致负数开方
    if variance.is_sign_negative() {
        Decimal::ZERO
    } else {
        variance.sqrt().unwrap_or(Decimal::ZERO)
    }
}

pub struct StdDev {
    buffer: RingBuffer<Decimal>,
    sum: Decimal,
    sum_squares: Decimal,
    period: Decimal,
    current: Decimal,
}

impl StdDev {
    pub fn new(period: usize) -> Self {
        Self {
            buffer: RingBuffer::new(period),
            sum: Decimal::ZERO,
            sum_squares: Decimal::ZERO,
            period: Decimal::from(period),
            current: Decimal::ZERO,
        }
    }
}

impl Indicator for StdDev {
    type State = Decimal;
    type Item<'a>
        = Decimal
    where
        Self: 'a;
    type Value = Decimal;

    fn state(&self) -> Option<&Self::State> {
        if self.buffer.is_full() {
            Some(&self.current)
        } else {
            None
        }
    }

    fn calc(&self, next: Self::Item<'_>) -> Option<Self::Value> {
        // 只有在buffer已满时才能计算标准差
        if !self.buffer.is_full() {
            return None;
        }

        // 获取即将被替换的值（最老的值）
        let removed_value = self.buffer.get(0)?;

        // 计算移除旧值并添加新值后的统计量
        let new_sum = self.sum - removed_value + next;
        let new_sum_squares =
            self.sum_squares - removed_value.powi(2) + next.powi(2);

        // 计算新的标准差
        let new_std_dev =
            compute_std_dev(new_sum, new_sum_squares, self.period);

        Some(new_std_dev)
    }

    fn update(&mut self, next: Self::Item<'_>) -> Option<Self::Value> {
        // 更新buffer
        let removed = self.buffer.update(next);

        if self.buffer.is_full() {
            if let Some(removed_value) = removed {
                // 更新总和和平方和
                self.sum = self.sum - removed_value + next;
                self.sum_squares =
                    self.sum_squares - removed_value.powi(2) + next.powi(2);
            } else {
                // 第一次填满时计算初始值
                self.sum = self.buffer.iter().sum::<Decimal>();
                self.sum_squares =
                    self.buffer.iter().map(|x| x.powi(2)).sum::<Decimal>();
            }

            self.current =
                compute_std_dev(self.sum, self.sum_squares, self.period);

            Some(self.current)
        } else {
            // buffer不满时不返回任何值
            None
        }
    }
}
