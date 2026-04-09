use std::{borrow::Cow, str::FromStr};

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    config::ColorTable,
    impl_builder,
    monitor::{Msg, State, alert::AlertManager},
    prelude::*,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlertMode {
    Temp,
    Perm,
}

impl AlertMode {
    pub const TEMP_STR: &str = "temp";
    pub const PERM_STR: &str = "perm";

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Temp => Self::TEMP_STR,
            Self::Perm => Self::PERM_STR,
        }
    }
}

impl FromStr for AlertMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::TEMP_STR => Ok(Self::Temp),
            Self::PERM_STR => Ok(Self::Perm),
            other => Err(other.unexpected("parse alert mode")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiffArgs {
    pub alert_mode: AlertMode,
    pub threshold: f64,
}

impl FromStr for DiffArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut alert_mode_str = String::new();
        let mut threshold = 0.0f64;

        sscanf!(s, "pdiff:{alert_mode_str},{threshold}").with_context(
            |_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse diff args"),
            },
        )?;

        let alert_mode = alert_mode_str.parse()?;

        Ok(Self {
            alert_mode,
            threshold,
        })
    }
}

impl Args for DiffArgs {
    type Type = (AlertMode, f64);
    type Target = Diff;

    fn new(args: Self::Type) -> Self {
        Self {
            alert_mode: args.0,
            threshold: args.1,
        }
    }

    fn key(&self) -> String {
        format!("pdiff:{},{}", self.alert_mode.as_str(), self.threshold)
    }

    fn build(self) -> Result<Self::Target> {
        let hundred = Decimal::from(100);
        let threshold = Decimal::from_f64(self.threshold)
            .required("diff threshold")?
            / hundred;
        let key = self.key();

        Ok(Diff {
            args: self,
            threshold,
            hundred,
            key,
            state: Default::default(),
            alerts: Default::default(),
            prev_t: None,
        })
    }
}

impl_builder!(DiffBuilder: DiffArgs => Diff);

pub struct Diff {
    args: DiffArgs,
    threshold: Decimal,
    hundred: Decimal,
    key: String,
    state: State,
    alerts: AlertManager,
    prev_t: Option<OffsetDateTime>,
}

impl Diff {
    fn calc(&self, kctx: &KCtx) -> Option<Msg> {
        let open = kctx.info.raw.price_open;
        let close = kctx.info.raw.price_close;

        if open.is_zero() {
            return None;
        }

        let abs = (close - open).abs();
        let rate = abs / open;

        if rate < self.threshold {
            return None;
        }

        let is_up = close >= open;
        Some(self.format_msg(abs, rate, is_up, kctx.colors))
    }

    fn format_msg(
        &self,
        abs: Decimal,
        rate: Decimal,
        is_up: bool,
        colors: ColorTable,
    ) -> Msg {
        let diff_pct = (rate * self.hundred).round_dp(2);
        let diff_abs = abs.round_dp(2);
        let (sign, color) = if is_up {
            ("+", colors.up)
        } else {
            ("-", colors.down)
        };

        let desc = format!("{sign}{diff_pct}%({diff_abs})");
        let normal = format!("pdiff:{desc}");

        let tty = format!("pdiff:{}", desc.with(color));

        Msg { normal, tty }
    }
}

impl Monitor for Diff {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![]
    }

    fn apply(&mut self, kctx: &KCtx) {
        let msg_opt = self.calc(kctx);
        let t = kctx.info.raw.time_begin;
        let finalized = kctx.info.raw.finalized;
        let should_alert =
            finalized == (self.args.alert_mode == AlertMode::Perm);

        if let Some(msg) = msg_opt.as_ref()
            && should_alert
            && self.prev_t != Some(t)
        {
            self.prev_t.replace(t);
            self.alerts.add(t, msg.clone())
        }

        let state = msg_opt.map(|msg| (t, msg));

        if finalized {
            self.state.temp.take();
            self.state.perm = state;
        } else {
            self.state.temp = state;
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
