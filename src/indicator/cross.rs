use crate::prelude::Indicator;

#[derive(Clone)]
pub struct CrossItem<T> {
    pub fast: T,
    pub slow: T,
    pub direction: bool,
}

#[derive(Clone)]
pub struct CrossValue<T: Clone> {
    pub prev: CrossItem<T>,
    pub next: CrossItem<T>,
    pub cross: bool,
}

pub struct Cross<T: Clone> {
    prev: Option<CrossItem<T>>,
    state: Option<CrossValue<T>>,
}

impl<T: PartialEq + PartialOrd + Clone> Indicator for Cross<T> {
    type State = CrossValue<T>;
    type Item = CrossItem<T>;
    type Value = CrossValue<T>;

    fn state(&self) -> Option<&Self::State> {
        unimplemented!()
    }

    fn calc(&self, next: &Self::Item) -> Option<Self::Value> {
        unimplemented!()
    }

    fn update(&mut self, next: &Self::Item) -> Option<Self::Value> {
        unimplemented!()
    }
}
