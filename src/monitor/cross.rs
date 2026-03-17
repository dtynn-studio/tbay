use std::str::FromStr;

use crate::{
    impl_builder,
    indicator::cross::{CrossValue, MaCrossArgs},
    prelude::{Args, Builder, Decimal, Error, KCtx, Monitor, Result},
};

#[derive(Debug, Clone, Copy)]
pub struct CrossArgs(MaCrossArgs);

impl FromStr for CrossArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl Args for CrossArgs {
    type Type = MaCrossArgs;
    type Target = Cross;

    fn new(args: Self::Type) -> Self {
        Self(args)
    }

    fn key(&self) -> String {
        self.0.key()
    }

    fn build(self) -> Result<Self::Target> {
        let cross_key = self.0.key();
        let key = self.0.key();

        Ok(Cross {
            args: self,
            key,
            cross_key,
            current: None,
        })
    }
}

// cross & stay
pub struct Cross {
    args: CrossArgs,
    key: String,
    cross_key: String,
    current: Option<CrossValue<Decimal>>,
}

impl Cross {
    fn cross_event<'c>(
        &self,
        kctx: &'c KCtx,
    ) -> Option<(&'c CrossValue<Decimal>, String)> {
        let val = kctx.get_val::<CrossValue<Decimal>>(&self.cross_key)?;
        let direction = val.cross?;
        Some((val, self.cross_event_msg(direction)))
    }

    fn cross_event_msg(&self, direction: bool) -> String {
        let dir_flag = if direction { "↗" } else { "↘" };
        format!(
            "{}-{}:{}{dir_flag}{}",
            self.args.0.val_kind.as_str(),
            self.args.0.calc_kind.as_str(),
            self.args.0.fast,
            self.args.0.slow
        )
    }
}

impl_builder!(CrossBuilder: CrossArgs => Cross);

impl Monitor for Cross {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.cross_key]
    }

    fn calc(&self, kctx: &KCtx) -> Option<String> {
        self.cross_event(kctx).map(|(_, msg)| msg)
    }

    fn update(&mut self, kctx: &KCtx) -> Option<String> {
        let (val, msg) = self.cross_event(kctx)?;
        self.current.replace(val.clone());

        Some(msg)
    }

    fn terminated(&self) -> bool {
        false
    }
}
