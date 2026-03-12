use crate::{
    indicator::Indicator,
    prelude::{Decimal, KCtx},
};

pub struct Distance {
    key: String,
    key1: String,
    key2: String,
    current: Option<Decimal>,
}

impl Indicator for Distance {
    type Output = Decimal;

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.key1, &self.key2]
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let val1 = *next.get_val::<Decimal>(&self.key1)?;
        let val2 = *next.get_val::<Decimal>(&self.key2)?;

        Some((val1 - val2).abs())
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        let calculated = self.calc(next)?;
        self.current.replace(calculated);
        Some(calculated)
    }
}
