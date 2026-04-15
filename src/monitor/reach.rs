use std::borrow::Cow;

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    common::impl_builder,
    monitor::{Msg, State},
    prelude::*,
};

#[derive(Debug, Clone, Copy)]
pub struct ReachArgs {
    target: Decimal,
    direction: Option<bool>,
}

impl FromStr for ReachArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut target_str = String::new();
        let mut direction_str = String::new();

        if sscanf!(s, "reach:{target_str},{direction_str}").is_err() {
            target_str.clear();
            direction_str.clear();

            sscanf!(s, "reach:{target_str}").with_context(|_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse reach args"),
            })?;
        }

        let target =
            Decimal::from_str_radix(&target_str, 10).context(DecimalCtx {
                field: "reach args: target",
            })?;

        let direction = match direction_str.as_str() {
            "up" => Some(true),
            "down" => Some(false),
            "" => None,
            other => return Err(other.unexpected("direction str")),
        };

        Ok(Self { target, direction })
    }
}

impl Args for ReachArgs {
    type Type = (Decimal, Option<bool>);
    type Target = Reach;

    fn new(args: Self::Type) -> Self {
        Self {
            target: args.0,
            direction: args.1,
        }
    }

    fn key(&self) -> String {
        let direction_part = match self.direction {
            Some(true) => ",up",
            Some(false) => ",down",
            None => "",
        };

        format!("reach:{}{direction_part}", self.target)
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();
        Ok(Reach {
            args: self,
            key,
            reached: None,
            alerted: false,
            state: Default::default(),
        })
    }
}

impl_builder!(ReachBuilder: ReachArgs => Reach);

pub struct Reach {
    args: ReachArgs,
    key: String,
    reached: Option<(OffsetDateTime, Msg)>,
    alerted: bool,
    state: State,
}

impl Reach {
    fn check_reached(&self, kctx: &KCtx) -> Option<(Decimal, bool)> {
        if !kctx.info.full.contains(self.args.target) {
            return None;
        }

        let open = kctx.info.raw.price_open;

        if open <= self.args.target {
            Some((kctx.info.raw.price_high, true))
        } else {
            Some((kctx.info.raw.price_low, false))
        }
    }

    fn check_msg(&self, kctx: &KCtx) -> Option<Msg> {
        let (price, direction) = self.check_reached(kctx)?;

        let price_rounded = price.round_dp(2);
        let target_rounded = self.args.target.round_dp(2);

        let (dir_flag, dir_color) = if direction {
            ("↗", kctx.colors.up)
        } else {
            ("↘", kctx.colors.down)
        };

        let normal = format!("$:{price_rounded}{dir_flag}{target_rounded}");
        let tty = format!(
            "$:{price_rounded}{}{target_rounded}",
            dir_flag.with(dir_color)
        );

        Some(Msg { normal, tty })
    }
}

impl Monitor for Reach {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![]
    }

    fn apply(&mut self, kctx: &KCtx) {
        if self.reached.is_some() || self.alerted {
            return;
        }

        let t = kctx.info.raw.time_begin;
        let Some(msg) = self.check_msg(kctx) else {
            return;
        };

        self.reached.replace((t, msg));
    }

    fn state(&self) -> &State {
        &self.state
    }

    fn take_alerts(&mut self) -> Vec<(OffsetDateTime, Msg)> {
        if let Some(tmsg) = self.reached.take() {
            self.alerted = true;
            vec![tmsg]
        } else {
            vec![]
        }
    }

    fn terminated(&self) -> bool {
        self.alerted
    }

    fn is_once(&self) -> bool {
        true
    }
}
