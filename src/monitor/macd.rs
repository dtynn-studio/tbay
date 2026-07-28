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
    fn cross_event(&self, kctx: &KCtx, val: &MacdValue) -> Option<Msg> {
        if let Some(direction) = val.cross.as_ref().and_then(|c| c.cross) {
            let pos = !val.dif.is_sign_negative();
            return Some(self.cross_event_msg(direction, pos, kctx.colors));
        };

        let prev = self.current.as_ref()?;
        let prev_dif_positive = prev.dif.is_sign_positive();
        let prev_dea_positive = prev.dea.is_sign_positive();
        let dif_diff = prev_dea_positive != val.dif.is_sign_positive();
        let dea_diff = prev_dea_positive != val.dea.is_sign_positive();
        if dif_diff || dea_diff {
            Some(self.cross_zero_msg(
                kctx,
                dif_diff.then_some(prev_dif_positive),
                dea_diff.then_some(prev_dea_positive),
            ))
        } else {
            None
        }
    }

    fn cross_zero_msg(
        &self,
        kctx: &KCtx,
        dif_positive: Option<bool>,
        dea_positive: Option<bool>,
    ) -> Msg {
        let (fast, fast_color) = match dif_positive {
            Some(true) => ("F[↓]", kctx.colors.down),
            Some(false) => ("F[↑]", kctx.colors.up),
            None => ("", kctx.colors.normal),
        };

        let (slow, slow_color) = match dea_positive {
            Some(true) => ("S[↓]", kctx.colors.down),
            Some(false) => ("S[↑]", kctx.colors.up),
            None => ("", kctx.colors.normal),
        };

        let normal = format!("macd:0:{}{}", fast, slow);

        let tty = format!(
            "macd:0:{}{}",
            fast.with(fast_color),
            slow.with(slow_color),
        );

        Msg { normal, tty }
    }

    fn cross_event_msg(
        &self,
        direction: bool,
        pos: bool,
        colors: ColorTable,
    ) -> Msg {
        let (dir_flag, dir_color) = if direction {
            ("[↗]", colors.up)
        } else {
            ("[↘]", colors.down)
        };

        let (pos, pos_color) = if pos {
            ("▲", colors.up)
        } else {
            ("▼", colors.down)
        };

        let normal = format!("macd:{}{}", pos, dir_flag);

        let tty = format!(
            "macd:{}{}",
            pos.with(pos_color),
            dir_flag.with(dir_color),
        );

        Msg { normal, tty }
    }

    fn calc(&self, kctx: &KCtx) -> Option<Msg> {
        let val = kctx.get_val::<MacdValue>(&self.macd_key)?;
        self.cross_event(kctx, val)
    }

    fn update(&mut self, kctx: &KCtx) -> Option<Msg> {
        let val = kctx.get_val::<MacdValue>(&self.macd_key)?.clone();
        let msg = self.cross_event(kctx, &val);
        self.current.replace(val);
        msg
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
