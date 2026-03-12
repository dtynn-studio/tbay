use crate::{
    indicator::{
        cross::{Cross, CrossItem, CrossValue},
        ma::Ema,
        Calculator, Indicator, Indicator2,
    },
    prelude::{Decimal, KCtx},
};

#[derive(Clone)]
pub struct MacdValue {
    pub dif: Decimal,
    pub dea: Decimal,
    pub macd: Decimal,
    pub cross: Option<CrossValue<Decimal>>,
}

pub struct Macd {
    key: String,
    fast_ma_key: String,
    slow_ma_key: String,
    dea: Ema,
    current: Option<MacdValue>,
    cross: Cross<Decimal>,
}

impl Indicator for Macd {
    type Output = MacdValue;

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.fast_ma_key, &self.slow_ma_key]
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let fast = *next.get_val::<Decimal>(&self.fast_ma_key)?;
        let slow = *next.get_val::<Decimal>(&self.slow_ma_key)?;
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

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        let fast = *next.get_val::<Decimal>(&self.fast_ma_key)?;
        let slow = *next.get_val::<Decimal>(&self.slow_ma_key)?;
        let dif = fast - slow;
        let dea = self.dea.update(dif)?;

        let cross = self.cross.update(CrossItem::new(dif, dea));

        let value = MacdValue {
            dif,
            dea,
            macd: dif - dea,
            cross,
        };

        self.current.replace(value.clone());
        Some(value)
    }
}
