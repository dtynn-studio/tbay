use crate::{
    indicator::{
        cross::{Cross, CrossItem, CrossValue},
        ma::Ema,
    },
    prelude::{Arc, Decimal, Indicator, KSummary},
};

#[derive(Clone)]
pub struct MacdValue {
    pub dif: Decimal,
    pub dea: Decimal,
    pub macd: Decimal,
    pub cross: Option<CrossValue<Decimal>>,
}

pub struct Macd {
    fast_ma_key: String,
    slow_ma_key: String,
    dea: Ema,
    current: Option<MacdValue>,
    cross: Cross<Decimal>,
}

impl Indicator for Macd {
    type State = MacdValue;
    type Item = Arc<KSummary>;
    type Value = MacdValue;

    fn state(&self) -> Option<&Self::State> {
        self.current.as_ref()
    }

    fn calc(&self, next: Self::Item) -> Option<Self::Value> {
        let next = next.as_ref();
        let fast_opt = next.get_base(&self.fast_ma_key);
        let slow_opt = next.get_base(&self.slow_ma_key);

        let (fast, slow) = fast_opt.zip(slow_opt)?;
        let dif = fast - slow;
        let dea = self.dea.calc(dif)?;
        let cross = self.cross.calc(CrossItem::new(dif, dea));

        Some(MacdValue {
            dif,
            dea,
            macd: dif - dea,
            cross,
        })
    }

    fn update(&mut self, next: Self::Item) -> Option<Self::Value> {
        let next = next.as_ref();
        let fast_opt = next.get_base(&self.fast_ma_key);
        let slow_opt = next.get_base(&self.slow_ma_key);

        let (fast, slow) = fast_opt.zip(slow_opt)?;
        let dif = fast - slow;
        let dea = self.dea.update(dif)?;

        let cross = self.cross.update(CrossItem::new(dif, dea));

        self.current.replace(MacdValue {
            dif,
            dea,
            macd: dif - dea,
            cross,
        });

        self.current.clone()
    }
}
