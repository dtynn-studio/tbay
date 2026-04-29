use std::borrow::Cow;

use crate::{
    impl_builder,
    indicator::base::{BaseExtractorArgs, CalcKind, ExtractKind},
    prelude::*,
};

#[derive(Clone, Copy)]
pub struct Position2Args {
    calc_kind: CalcKind,
    base_periods: usize,
}

impl FromStr for Position2Args {
    type Err = Error;

    // key format: position:ema,20
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut calc_kind_str = String::new();
        let mut base_periods = 0usize;

        scanf::sscanf!(s, "position2:{calc_kind_str},{base_periods}")
            .with_context(|_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse Position2 args"),
            })?;

        let calc_kind = calc_kind_str.parse()?;

        if base_periods == 0 {
            return Err(base_periods.unexpected("position2 base period"));
        }

        Ok(Self {
            calc_kind,
            base_periods,
        })
    }
}

impl Args for Position2Args {
    type Type = (CalcKind, usize);
    type Target = Position2;

    fn new(args: Self::Type) -> Self {
        Self {
            calc_kind: args.0,
            base_periods: args.1,
        }
    }

    fn key(&self) -> String {
        format!(
            "position2:{},{}",
            self.calc_kind.as_str(),
            self.base_periods,
        )
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();

        let base_key = BaseExtractorArgs::new((
            ExtractKind::PriceClose,
            self.calc_kind,
            self.base_periods,
        ))
        .key();

        Ok(Position2 {
            key,
            base_key,
            state: None,
        })
    }
}

impl_builder!(Position2Builder: Position2Args => Position2);

#[derive(Clone, Copy, PartialEq)]
pub enum Pos {
    Above,
    Below,
    Chaos,
}

impl Pos {
    pub fn flag(self) -> &'static str {
        match self {
            Self::Above => "▲",
            Self::Below => "▼",
            Self::Chaos => "~",
        }
    }
}

impl Pos {
    fn detect(kctx: &KCtx, base: Decimal) -> Self {
        if kctx.info.body.low >= base && kctx.info.full.mid >= base {
            Self::Above
        } else if kctx.info.body.high <= base && kctx.info.full.mid <= base {
            Self::Below
        } else {
            Self::Chaos
        }
    }
}

#[derive(Clone, Copy)]
pub struct PosState {
    pub pos: Pos,
    pub periods: usize,
}

pub struct Position2 {
    key: String,
    base_key: String,
    state: Option<PosState>,
}

impl Indicator for Position2 {
    type Output = PosState;

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.base_key]
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let base = next.get_val::<Decimal>(&self.base_key).copied()?;
        let pos = Pos::detect(next, base);
        let Some(mut prev) = self.state else {
            return Some(PosState { pos, periods: 1 });
        };

        if prev.pos == pos {
            prev.periods += 1;
        } else {
            prev.pos = pos;
            prev.periods = 1;
        }

        Some(prev)
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        let base = next.get_val::<Decimal>(&self.base_key).copied()?;
        let pos = Pos::detect(next, base);

        if let Some(prev) = self.state.as_mut()
            && prev.pos == pos
        {
            prev.periods += 1;
        } else {
            self.state.replace(PosState { pos, periods: 1 });
        }

        self.state
    }
}
