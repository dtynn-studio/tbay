use crate::{
    indicator::Indicator,
    prelude::{Decimal, KCtx, KInfo},
};

#[derive(Clone, Copy)]
pub struct PositionValue {
    pub state: PositionState,
    pub flip: bool, // 是否发生翻转
}

#[derive(Clone, Copy)]
pub struct PositionState {
    pub position: bool,    // 前一个位置
    pub duration: usize,   // 持续周期数
    pub extremum: Decimal, // 极值
}

impl PositionState {
    fn new(k: &KInfo, base: Decimal) -> Self {
        let position = k.is_not_below(base);
        Self {
            position,
            duration: 1,
            extremum: k.full.extremum(position),
        }
    }

    pub fn update(&mut self, next: &KInfo, base: Decimal) -> bool {
        let position = next.is_not_below(base);
        let maybe = next.full.extremum(position);
        if position == self.position {
            self.duration += 1;
            if (maybe > self.extremum) == position {
                self.extremum = maybe;
            }
            false
        } else {
            self.position = position;
            self.duration = 1;
            self.extremum = maybe;
            true
        }
    }
}

pub struct Position {
    key: String,
    base_key: String,
    state: Option<PositionState>,
}

impl Position {
    pub fn new(key: &str, base_key: &str) -> Self {
        Self {
            key: key.to_string(),
            base_key: base_key.to_string(),
            state: None,
        }
    }
}

impl Indicator for Position {
    type Output = PositionValue;

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.base_key]
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let base = *next.get_val::<Decimal>(&self.base_key)?;
        let Some(mut state) = self.state else {
            return Some(PositionValue {
                state: PositionState::new(&next.info, base),
                flip: false,
            });
        };

        let flip = state.update(&next.info, base);

        Some(PositionValue { state, flip })
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        let calculated = self.calc(next)?;
        self.state.replace(calculated.state);
        Some(calculated)
    }
}
