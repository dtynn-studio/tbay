use std::{borrow::Cow, str::FromStr};

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    config::ColorTable,
    impl_builder,
    indicator::bollinger::{BollingerBandArgs, BollingerBandValue},
    monitor::{Msg, alert::AlertManager},
    prelude::*,
};

#[derive(Debug, Clone, Copy)]
pub struct BbArgs {
    pub period: usize,
    pub width: usize,
}

impl FromStr for BbArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut period = 0usize;
        let mut width = 0usize;

        sscanf!(s, "bb:{period},{width}").with_context(|_| ParseCtx {
            raw: s.to_owned(),
            usage: Cow::from("parse bb args"),
        })?;

        if period == 0 {
            return Err(period.unexpected("bb period"));
        }

        if width == 0 {
            return Err(width.unexpected("bb width"));
        }

        Ok(Self { period, width })
    }
}

impl Args for BbArgs {
    type Type = (usize, usize);
    type Target = Bb;

    fn new(args: Self::Type) -> Self {
        Self {
            period: args.0,
            width: args.1,
        }
    }

    fn key(&self) -> String {
        format!("bb:{},{}", self.period, self.width)
    }

    fn build(self) -> Result<Self::Target> {
        let bb_args = BollingerBandArgs::new((self.period, self.width));
        let bb_key = bb_args.key();
        let key = self.key();

        Ok(Bb {
            args: self,
            key,
            bb_key,
            prev_touched_up: false,
            prev_touched_low: false,
            state: Default::default(),
            alerts: Default::default(),
            temp_t: None,
        })
    }
}

impl_builder!(BbBuilder: BbArgs => Bb);

pub struct Bb {
    args: BbArgs,
    key: String,
    bb_key: String,
    prev_touched_up: bool,
    prev_touched_low: bool,
    state: State,
    alerts: AlertManager,
    temp_t: Option<OffsetDateTime>,
}

impl Bb {
    fn touched(&self, kctx: &KCtx, bb: &BollingerBandValue) -> Option<bool> {
        let price_high = kctx.info.raw.price_high;
        let price_low = kctx.info.raw.price_low;

        let touched_up = price_high >= bb.up;
        let touched_low = price_low <= bb.low;

        if touched_up || touched_low {
            Some(
                touched_up && !self.prev_touched_up
                    || touched_low && !self.prev_touched_low,
            )
        } else {
            Some(false)
        }
    }

    fn event_msg(&self, touched_up: bool, colors: ColorTable) -> Msg {
        let (dir_str, color) = if touched_up {
            ("↑", colors.up)
        } else {
            ("↓", colors.down)
        };

        let normal = format!(
            "(bb/{}/{}):{}",
            self.args.period, self.args.width, dir_str,
        );

        let tty = format!(
            "(bb/{}/{}):{}",
            self.args.period,
            self.args.width,
            dir_str.with(color),
        );

        Msg { normal, tty }
    }

    fn calc(&self, kctx: &KCtx) -> Option<Msg> {
        if self.prev_touched_up || self.prev_touched_low {
            return None;
        }

        let bb = kctx.get_val::<BollingerBandValue>(&self.bb_key)?;

        let touched = self.touched(kctx, bb)?;
        if !touched {
            return None;
        }

        let touched_up = kctx.info.raw.price_high >= bb.up;
        Some(self.event_msg(touched_up, kctx.colors))
    }

    fn update(&mut self, kctx: &KCtx) -> Option<Msg> {
        let prev_touched = self.prev_touched_up || self.prev_touched_low;
        self.prev_touched_up = false;
        self.prev_touched_low = false;

        if prev_touched {
            return None;
        }

        let bb = kctx.get_val::<BollingerBandValue>(&self.bb_key)?;

        let touched = self.touched(kctx, bb)?;
        if !touched {
            return None;
        }

        let touched_up = kctx.info.raw.price_high >= bb.up;
        if touched_up {
            self.prev_touched_up = true;
        } else {
            self.prev_touched_low = true;
        }

        Some(self.event_msg(touched_up, kctx.colors))
    }
}

impl Monitor for Bb {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.bb_key]
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
