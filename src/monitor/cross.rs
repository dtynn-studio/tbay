use std::str::FromStr;

use time::OffsetDateTime;

use crate::{
    impl_builder,
    indicator::cross::{CrossValue, MaCrossArgs},
    prelude::{Args, Builder, Decimal, Error, KCtx, Monitor, Result, State},
    util::time::format_hhmm,
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
            state: Default::default(),
            temp_t: None,
        })
    }
}

impl_builder!(CrossBuilder: CrossArgs => Cross);

// cross & stay
pub struct Cross {
    args: CrossArgs,
    key: String,
    cross_key: String,
    current: Option<CrossValue<Decimal>>,
    state: State,
    temp_t: Option<OffsetDateTime>,
}

impl Cross {
    fn cross_event<'c>(
        &self,
        kctx: &'c KCtx,
    ) -> Option<(&'c CrossValue<Decimal>, String)> {
        let val = kctx.get_val::<CrossValue<Decimal>>(&self.cross_key)?;
        let direction = val.cross?;
        Some((
            val,
            self.cross_event_msg(&kctx.info.raw.time_begin, direction),
        ))
    }

    fn cross_event_msg(&self, t: &OffsetDateTime, direction: bool) -> String {
        let dir_flag = if direction { "↗" } else { "↘" };
        format!(
            "{}:({}/{}):{}{dir_flag}{}",
            format_hhmm(t),
            self.args.0.val_kind.as_str_short(),
            self.args.0.calc_kind.as_str_short(),
            self.args.0.fast,
            self.args.0.slow
        )
    }

    fn calc(&self, kctx: &KCtx) -> Option<String> {
        self.cross_event(kctx).map(|(_, msg)| msg)
    }

    fn update(&mut self, kctx: &KCtx) -> Option<String> {
        let (val, msg) = self.cross_event(kctx)?;
        self.current.replace(val.clone());

        Some(msg)
    }
}

impl Monitor for Cross {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.cross_key]
    }

    fn apply(&mut self, kctx: &KCtx) {
        if kctx.info.raw.finalized {
            self.state.temp.take();
            self.state.perm = self.update(kctx);
        } else {
            let prev_temp_t = self.temp_t.replace(kctx.info.raw.time_begin);
            if prev_temp_t != self.temp_t || self.state.temp.is_none() {
                self.state.temp = self.calc(kctx);
            }
        }
    }

    fn state(&self) -> &State {
        &self.state
    }

    fn terminated(&self) -> bool {
        false
    }
}
