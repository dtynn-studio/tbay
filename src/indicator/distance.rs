use crate::prelude::{Decimal, Indicator, KSummary};

pub struct Distance {
    key1: String,
    key2: String,
    current: Option<Decimal>,
}

impl Indicator for Distance {
    type State = Decimal;
    type Item<'a> = &'a KSummary;
    type Value = Decimal;

    fn state(&self) -> Option<&Self::State> {
        self.current.as_ref()
    }

    fn calc<'a>(&self, next: Self::Item<'a>) -> Option<Self::Value> {
        let val1 = next.get_base(&self.key1)?;
        let val2 = next.get_base(&self.key2)?;

        Some((val1 - val2).abs())
    }

    fn update<'a>(&mut self, next: Self::Item<'a>) -> Option<Self::Value> {
        let calculated = self.calc(next)?;
        self.current.replace(calculated);
        Some(calculated)
    }
}
