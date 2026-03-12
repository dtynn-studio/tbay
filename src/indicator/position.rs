use crate::prelude::{Arc, Decimal, Indicator2, KSummary};

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
    fn new(k: &KSummary, base: Decimal) -> Self {
        let position = k.info.is_not_below(base);
        Self {
            position,
            duration: 1,
            extremum: k.info.full.extremum(position),
        }
    }

    pub fn update(&mut self, next: &KSummary, base: Decimal) -> bool {
        let position = next.info.is_not_below(base);
        let maybe = next.info.full.extremum(position);
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

impl Indicator2 for Position {
    type State = PositionState;
    type Item = Arc<KSummary>;
    type Value = PositionValue;

    fn key(&self) -> &str {
        &self.key
    }

    fn state(&self) -> Option<&Self::State> {
        self.state.as_ref()
    }

    fn calc(&self, next: Self::Item) -> Option<PositionValue> {
        let next = next.as_ref();
        // 1. 获取基线值
        let base = next.get_base(&self.base_key)?;
        let Some(mut state) = self.state else {
            return Some(PositionValue {
                state: PositionState::new(next, base),
                flip: false,
            });
        };

        // 2. 获取当前K线的相对位置
        let flip = state.update(next, base);

        Some(PositionValue { state, flip })
    }

    fn update(&mut self, next: Self::Item) -> Option<PositionValue> {
        // 第一次更新：初始化状态
        // 只有 base 为 None 的情况才会值为 None
        let calculated = self.calc(next)?;
        self.state.replace(calculated.state);
        Some(calculated)
    }

    fn deps(&self) -> Vec<String> {
        vec![self.base_key.clone()]
    }
}
