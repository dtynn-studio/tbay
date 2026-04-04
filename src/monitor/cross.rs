use std::str::FromStr;

use crossterm::style::Stylize;
use time::OffsetDateTime;

use crate::{
    config::ColorTable,
    impl_builder,
    indicator::cross::{CrossValue, MaCrossArgs},
    monitor::{Msg, alert::AlertManager},
    prelude::{Args, Builder, Decimal, Error, KCtx, Monitor, Result, State},
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
            alerts: Default::default(),
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
    alerts: AlertManager,
    temp_t: Option<OffsetDateTime>,
}

impl Cross {
    fn cross_event<'c>(
        &self,
        kctx: &'c KCtx,
    ) -> Option<(&'c CrossValue<Decimal>, Msg)> {
        let val = kctx.get_val::<CrossValue<Decimal>>(&self.cross_key)?;
        let direction = val.cross?;
        Some((val, self.cross_event_msg(direction, kctx.colors)))
    }

    fn cross_event_msg(&self, direction: bool, colors: ColorTable) -> Msg {
        let (dir_flag, color) = if direction {
            ("↗", colors.up)
        } else {
            ("↘", colors.down)
        };
        let normal = format!(
            "({}/{}):{}{dir_flag}{}",
            self.args.0.val_kind.as_str_short(),
            self.args.0.calc_kind.as_str_short(),
            self.args.0.fast,
            self.args.0.slow
        );

        let tty = format!(
            "({}/{}):{}{}{}",
            self.args.0.val_kind.as_str_short(),
            self.args.0.calc_kind.as_str_short(),
            self.args.0.fast,
            dir_flag.with(color),
            self.args.0.slow
        );

        Msg { normal, tty }
    }

    fn calc(&self, kctx: &KCtx) -> Option<Msg> {
        self.cross_event(kctx).map(|(_, msg)| msg)
    }

    fn update(&mut self, kctx: &KCtx) -> Option<Msg> {
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

    fn take_alerts(&mut self) -> Vec<(OffsetDateTime, Msg)> {
        self.alerts.take()
    }

    fn terminated(&self) -> bool {
        false
    }
}
