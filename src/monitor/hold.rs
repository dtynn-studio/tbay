use std::{borrow::Cow, str::FromStr};

use scanf::sscanf;

use crate::{
    impl_builder,
    indicator::{
        base::{CalcKind, ExtractKind},
        position::{PositionArgs, PositionValue},
    },
    monitor::alert::AlertManager,
    prelude::*,
};

#[derive(Debug, Clone, Copy)]
pub struct HoldArgs {
    pub val_kind: ExtractKind,
    pub calc_kind: CalcKind,
    pub ma: usize,
    pub hold: usize,
}

impl FromStr for HoldArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut val_kind_str = String::new();
        let mut calc_kind_str = String::new();
        let mut ma = 0usize;
        let mut hold = 0usize;

        sscanf!(s, "hold:{val_kind_str},{calc_kind_str},{ma},{hold}")
            .with_context(|_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse hold args"),
            })?;

        let val_kind = val_kind_str.parse()?;
        let calc_kind = calc_kind_str.parse()?;

        if ma == 0 {
            return Err(ma.unexpected("ma period"));
        }

        if hold == 0 {
            return Err(ma.unexpected("hold period"));
        }

        Ok(Self {
            val_kind,
            calc_kind,
            ma,
            hold,
        })
    }
}

impl Args for HoldArgs {
    type Type = (ExtractKind, CalcKind, usize, usize);
    type Target = Hold;

    fn new(args: Self::Type) -> Self {
        Self {
            val_kind: args.0,
            calc_kind: args.1,
            ma: args.2,
            hold: args.3,
        }
    }

    fn key(&self) -> String {
        format!(
            "hold:{},{},{},{}",
            self.val_kind.as_str(),
            self.calc_kind.as_str(),
            self.ma,
            self.hold
        )
    }

    fn build(self) -> Result<Self::Target> {
        let pos_args =
            PositionArgs::new((self.val_kind, self.calc_kind, self.ma));
        let pos_key = pos_args.key();
        let key = self.key();

        Ok(Hold {
            args: self,
            key,
            pos_key,
            state: Default::default(),
            alerts: Default::default(),
            temp_t: None,
        })
    }
}

impl_builder!(HoldBuilder: HoldArgs => Hold);

pub struct Hold {
    args: HoldArgs,
    key: String,
    pos_key: String,
    state: State,
    alerts: AlertManager,
    temp_t: Option<OffsetDateTime>,
}

impl Hold {
    fn event_msg(&self, pos: bool) -> String {
        let pos_str = if pos { "▲" } else { "▼" };

        format!(
            "({}/{}{}):{pos_str}{}",
            self.args.val_kind.as_str_short(),
            self.args.calc_kind.as_str_short(),
            self.args.ma,
            self.args.hold
        )
    }

    fn calc(&self, kctx: &KCtx) -> Option<String> {
        let val = kctx.get_val::<PositionValue>(&self.pos_key)?;
        if val.state.duration != self.args.hold {
            return None;
        }

        Some(self.event_msg(val.state.position))
    }

    fn update(&mut self, kctx: &KCtx) -> Option<String> {
        self.calc(kctx)
    }
}

impl Monitor for Hold {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.pos_key]
    }

    fn apply(&mut self, kctx: &KCtx) {
        if kctx.info.raw.finalized {
            self.state.temp.take();
            self.state.perm =
                self.update(kctx).map(|msg| (kctx.info.raw.time_begin, msg));
        } else {
            let prev_temp_t = self.temp_t.replace(kctx.info.raw.time_begin);
            if prev_temp_t != self.temp_t || self.state.temp.is_none() {
                self.state.temp =
                    self.calc(kctx).map(|msg| (kctx.info.raw.time_begin, msg));

                if let Some((t, msg)) = self.state.temp.as_ref().cloned() {
                    self.alerts.add(t, msg);
                }
            }
        }
    }

    fn state(&self) -> &State {
        &self.state
    }

    fn take_alerts(&mut self) -> Vec<(OffsetDateTime, String)> {
        vec![]
    }

    fn terminated(&self) -> bool {
        false
    }
}
