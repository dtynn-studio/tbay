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
            _args: self,
            key,
            bb_key,
            state: Default::default(),
            alerts: Default::default(),
            temp_t: None,
        })
    }
}

impl_builder!(BbBuilder: BbArgs => Bb);

pub struct Bb {
    _args: BbArgs,
    key: String,
    bb_key: String,
    state: State,
    alerts: AlertManager,
    temp_t: Option<OffsetDateTime>,
}

impl Bb {
    fn touched(&self, kctx: &KCtx, bb: &BollingerBandValue) -> Option<bool> {
        let price_high = kctx.info.raw.price_high;
        let price_low = kctx.info.raw.price_low;

        let touched_up = price_high >= bb.up;
        let touched_down = price_low <= bb.low;

        if !touched_up && !touched_down {
            return None;
        }

        Some(touched_up)
    }

    fn event_msg(&self, direction: bool, colors: ColorTable) -> Msg {
        let (dir_str, color) = if direction {
            ("[↑]", colors.up)
        } else {
            ("[↓]", colors.down)
        };

        let normal = format!("bb:{}", dir_str,);

        let tty = format!("bb:{}", dir_str.with(color),);

        Msg { normal, tty }
    }

    fn calc(&self, kctx: &KCtx) -> Option<Msg> {
        let bb = kctx.get_val::<BollingerBandValue>(&self.bb_key)?;
        let direction = self.touched(kctx, bb)?;

        Some(self.event_msg(direction, kctx.colors))
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
                self.calc(kctx).map(|msg| (kctx.info.raw.time_begin, msg));
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

    fn is_once(&self) -> bool {
        false
    }
}
