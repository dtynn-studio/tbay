use std::str::FromStr;

use crossterm::style::Stylize;
use time::OffsetDateTime;

use crate::{
    config::ColorTable,
    impl_builder,
    indicator::macd::{MacdArgs, MacdValue},
    monitor::{Msg, alert::AlertManager},
    prelude::{Args, Builder, Error, KCtx, Monitor, Result, State},
};

#[derive(Debug, Clone, Copy)]
pub struct MacdMonitorArgs(MacdArgs);

impl FromStr for MacdMonitorArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl Args for MacdMonitorArgs {
    type Type = MacdArgs;
    type Target = MacdMonitor;

    fn new(args: Self::Type) -> Self {
        Self(args)
    }

    fn key(&self) -> String {
        self.0.key()
    }

    fn build(self) -> Result<Self::Target> {
        let macd_key = self.0.key();
        let key = self.0.key();

        Ok(MacdMonitor {
            _args: self,
            key,
            macd_key,
            current: None,
            state: Default::default(),
            alerts: Default::default(),
        })
    }
}

impl_builder!(MacdMonitorBuilder: MacdMonitorArgs => MacdMonitor);

pub struct MacdMonitor {
    _args: MacdMonitorArgs,
    key: String,
    macd_key: String,
    current: Option<MacdValue>,
    state: State,
    alerts: AlertManager,
}

impl MacdMonitor {
    fn cross_event(&self, kctx: &KCtx) -> Option<(bool, Msg)> {
        let val = kctx.get_val::<MacdValue>(&self.macd_key)?;
        let direction = val.cross.clone()?.cross?;
        Some((direction, self.cross_event_msg(direction, kctx.colors)))
    }

    fn cross_event_msg(&self, direction: bool, colors: ColorTable) -> Msg {
        let (dir_flag, color) = if direction {
            ("[↗]", colors.up)
        } else {
            ("[↘]", colors.down)
        };

        let normal = format!("macd:{}", dir_flag);

        let tty = format!("macd:{}", dir_flag.with(color),);

        Msg { normal, tty }
    }

    fn calc(&self, kctx: &KCtx) -> Option<Msg> {
        self.cross_event(kctx).map(|(_, msg)| msg)
    }

    fn update(&mut self, kctx: &KCtx) -> Option<Msg> {
        let (_direction, msg) = self.cross_event(kctx)?;
        let val = kctx.get_val::<MacdValue>(&self.macd_key)?.clone();
        self.current.replace(val);
        Some(msg)
    }
}

impl Monitor for MacdMonitor {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.macd_key]
    }

    fn apply(&mut self, kctx: &KCtx) {
        if kctx.info.raw.finalized {
            self.state.temp.take();
            self.state.perm =
                self.update(kctx).map(|msg| (kctx.info.raw.time_begin, msg));
            if let Some((t, msg)) = self.state.perm.as_ref().cloned() {
                self.alerts.add(t, msg);
            }
        } else {
            self.state.temp =
                self.calc(kctx).map(|msg| (kctx.info.raw.time_begin, msg));
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

    fn is_once(&self) -> bool {
        false
    }
}
