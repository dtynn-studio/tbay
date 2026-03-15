use crate::{
    indicator::{
        Indicator,
        base::{BaseExtractorArgs, CalcKind, ExtractKind},
    },
    prelude::{
        Args, Decimal, Error, FromStr, KCtx, KInfo, ParseCtx, Result,
        Unexpected,
    },
};
use snafu::ResultExt;
use std::borrow::Cow;

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
    // close:ema:{base}
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

#[derive(Debug, Clone, Copy)]
pub struct PositionArgs {
    kind: CalcKind,
    base: usize,
}

impl FromStr for PositionArgs {
    type Err = Error;

    // key format: position:ema,20
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut kind_str = String::new();
        let mut base = 0usize;

        scanf::sscanf!(s, "position:{kind_str},{base}").with_context(|_| {
            ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse Position"),
            }
        })?;

        let kind = match kind_str.as_str() {
            "sma" => CalcKind::Sma,
            "ema" => CalcKind::Ema,
            other => return Err(other.unexpected("position kind")),
        };

        if base == 0 {
            return Err(base.unexpected("position base period"));
        }

        Ok(Self { kind, base })
    }
}

impl Args for PositionArgs {
    type Type = (CalcKind, usize);
    type Target = Position;

    fn new(args: Self::Type) -> Self {
        Self {
            kind: args.0,
            base: args.1,
        }
    }

    fn key(&self) -> String {
        format!("position:{},{}", self.kind.as_str(), self.base)
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();

        let base_key = BaseExtractorArgs::new((
            ExtractKind::PriceClose,
            self.kind,
            self.base,
        ))
        .key();

        Ok(Position::new(&key, &base_key))
    }
}
