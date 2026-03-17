use std::{borrow::Cow, str::FromStr};

use scanf::sscanf;

use crate::{
    impl_builder,
    indicator::{
        base::{CalcKind, ExtractKind},
        position::{PositionArgs, PositionValue},
    },
    prelude::*,
};

#[derive(Debug, Clone, Copy)]
pub struct HoldArgs {
    pub calc_kind: CalcKind,
    pub ma: usize,
    pub hold: usize,
}

impl FromStr for HoldArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut calc_kind_str = String::new();
        let mut ma = 0usize;
        let mut hold = 0usize;

        sscanf!(s, "hold:{calc_kind_str},{ma},{hold}").with_context(|_| {
            ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse hold args"),
            }
        })?;

        let calc_kind = calc_kind_str.parse()?;

        if ma == 0 {
            return Err(ma.unexpected("ma period"));
        }

        if hold == 0 {
            return Err(ma.unexpected("hold period"));
        }

        Ok(Self {
            calc_kind,
            ma,
            hold,
        })
    }
}

impl Args for HoldArgs {
    type Type = (CalcKind, usize, usize);
    type Target = Hold;

    fn new(args: Self::Type) -> Self {
        Self {
            calc_kind: args.0,
            ma: args.1,
            hold: args.2,
        }
    }

    fn key(&self) -> String {
        format!("hold:{},{},{}", self.calc_kind.as_str(), self.ma, self.hold)
    }

    fn build(self) -> Result<Self::Target> {
        let pos_args = PositionArgs::new((self.calc_kind, self.ma));
        let pos_key = pos_args.key();
        let key = self.key();

        Ok(Hold {
            args: self,
            key,
            pos_key,
        })
    }
}

pub struct Hold {
    args: HoldArgs,
    key: String,
    pos_key: String,
}

impl Hold {
    fn event_msg(&self) -> String {
        format!(
            "{}:{}{}@{}",
            ExtractKind::PriceClose.as_str(),
            self.args.calc_kind.as_str(),
            self.args.ma,
            self.args.hold
        )
    }
}

impl Monitor for Hold {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.pos_key]
    }

    fn calc(&self, kctx: &KCtx) -> Option<String> {
        let val = kctx.get_val::<PositionValue>(&self.pos_key)?;
        if val.state.duration != self.args.hold {
            return None;
        }

        Some(self.event_msg())
    }

    fn update(&mut self, kctx: &KCtx) -> Option<String> {
        self.calc(kctx)
    }

    fn terminated(&self) -> bool {
        false
    }
}

impl_builder!(HoldBuilder: HoldArgs => Hold);
