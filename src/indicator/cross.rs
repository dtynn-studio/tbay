use std::cmp::Ordering;

use crate::prelude::Indicator2;

#[derive(Clone)]
pub struct CrossItem<T> {
    pub fast: T,
    pub slow: T,
    pub pos: Ordering,
}

impl<T: Ord + Clone> CrossItem<T> {
    pub fn new(fast: T, slow: T) -> Self {
        let pos = fast.cmp(&slow);
        Self { fast, slow, pos }
    }
}

#[derive(Clone)]
pub struct CrossValue<T: Clone> {
    pub prev: CrossItem<T>,
    pub next: CrossItem<T>,
    // 上穿为 Some(true),
    // 下穿为 Some(false)
    // 未发生穿越则为 None
    pub cross: Option<bool>,
}

fn calc_cross<T: Ord>(
    prev: &CrossItem<T>,
    next: &CrossItem<T>,
) -> Option<bool> {
    match (prev.pos, next.pos) {
        // 在同侧
        (Ordering::Greater, Ordering::Greater)
        | (Ordering::Less, Ordering::Less) => None,

        (Ordering::Equal, Ordering::Equal) => match next.fast.cmp(&prev.fast) {
            Ordering::Greater => Some(true),
            Ordering::Less => Some(false),
            Ordering::Equal => None,
        },

        (Ordering::Less, Ordering::Equal)
        | (Ordering::Less, Ordering::Greater)
        | (Ordering::Equal, Ordering::Greater) => Some(true),

        (Ordering::Greater, Ordering::Equal)
        | (Ordering::Greater, Ordering::Less)
        | (Ordering::Equal, Ordering::Less) => Some(false),
    }
}

#[derive(Clone)]
pub struct Cross<T: Clone> {
    key: String,
    prev: Option<CrossItem<T>>,
    state: Option<CrossValue<T>>,
}

impl<T: Ord + Clone + 'static> Indicator2 for Cross<T> {
    type State = CrossValue<T>;
    type Item = CrossItem<T>;
    type Value = CrossValue<T>;

    fn key(&self) -> &str {
        &self.key
    }

    fn state(&self) -> Option<&Self::State> {
        self.state.as_ref()
    }

    fn calc(&self, next: Self::Item) -> Option<Self::Value> {
        let prev = self.prev.as_ref()?;

        let cross = calc_cross(prev, &next);

        Some(CrossValue {
            prev: prev.clone(),
            next,
            cross,
        })
    }

    fn update(&mut self, next: Self::Item) -> Option<Self::Value> {
        let prev = match self.prev.take() {
            Some(p) => {
                self.prev.replace(next.clone());
                p
            }

            None => {
                self.prev.replace(next);
                return None;
            }
        };

        let cross = calc_cross(&prev, &next);
        let value = CrossValue { prev, next, cross };

        Some(value)
    }

    fn deps(&self) -> Vec<String> {
        vec![]
    }
}
