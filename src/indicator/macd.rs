use crate::{
    indicator::ma::Ema,
    prelude::{BaseKind, Decimal, Indicator, KSummary},
};

#[derive(Debug, Clone, Copy)]
pub struct MacdValue {
    pub dif: Decimal,
    pub dea: Decimal,
    pub macd: Decimal,
}

pub struct Macd {
    fast_ma_key: String,
    slow_ma_key: String,
    dea: Ema,
    current: Option<MacdValue>,
}

impl Indicator for Macd {
    type State = MacdValue;
    type Item = KSummary;
    type Value = MacdValue;

    fn state(&self) -> Option<&Self::State> {
        self.current.as_ref()
    }

    fn calc(&self, next: &Self::Item) -> Option<Self::Value> {
        let fast_opt = next.get_base(BaseKind::Price, &self.fast_ma_key);
        let slow_opt = next.get_base(BaseKind::Price, &self.slow_ma_key);

        let (fast, slow) = fast_opt.zip(slow_opt)?;
        let dif = fast - slow;
        let dea = self.dea.calc(&dif)?;

        Some(MacdValue {
            dif,
            dea,
            macd: dif - dea,
        })
    }

    fn update(&mut self, next: &Self::Item) -> Option<Self::Value> {
        let fast_opt = next.get_base(BaseKind::Price, &self.fast_ma_key);
        let slow_opt = next.get_base(BaseKind::Price, &self.slow_ma_key);

        let (fast, slow) = fast_opt.zip(slow_opt)?;
        let dif = fast - slow;
        let dea = self.dea.update(&dif)?;

        self.current.replace(MacdValue {
            dif,
            dea,
            macd: dif - dea,
        });

        self.current
    }
}
