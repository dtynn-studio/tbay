use crate::prelude::{Arc, Decimal, Indicator, KSummary};

pub struct Distance {
    key1: String,
    key2: String,
    current: Option<Decimal>,
}

impl Indicator for Distance {
    type State = Decimal;
    type Item = Arc<KSummary>;
    type Value = Decimal;

    fn state(&self) -> Option<&Self::State> {
        self.current.as_ref()
    }

    fn calc(&self, next: Self::Item) -> Option<Self::Value> {
        let next = next.as_ref();
        let val1 = next.get_base(&self.key1)?;
        let val2 = next.get_base(&self.key2)?;

        Some((val1 - val2).abs())
    }

    fn update(&mut self, next: Self::Item) -> Option<Self::Value> {
        let calculated = self.calc(next)?;
        self.current.replace(calculated);
        Some(calculated)
    }

    fn deps(&self) -> Vec<String> {
        vec![self.key1.clone(), self.key2.clone()]
    }
}
