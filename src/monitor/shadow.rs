use std::borrow::Cow;

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    config::ColorTable,
    impl_builder,
    monitor::{Msg, alert::AlertManager},
    prelude::*,
};

#[derive(Debug, Clone, Copy)]
pub struct ShadowArgs {
    threshold: Decimal,
}

impl FromStr for ShadowArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut threshold_f = 0.0f64;

        sscanf!(s, "shadow:{threshold_f}").with_context(|_| ParseCtx {
            raw: s.to_owned(),
            usage: Cow::from("parse shadow args"),
        })?;

        let threshold =
            Decimal::from_f64(threshold_f).required("shadow threshold")?;

        Ok(Self { threshold })
    }
}

impl Args for ShadowArgs {
    type Type = Decimal;
    type Target = Shadow;

    fn new(threshold: Self::Type) -> Self {
        Self { threshold }
    }

    fn key(&self) -> String {
        format!("shadow:{}", self.threshold)
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();

        Ok(Shadow {
            args: self,
            key,
            state: Default::default(),
            alerts: Default::default(),
        })
    }
}

impl_builder!(ShadowBuilder: ShadowArgs => Shadow);

pub struct Shadow {
    args: ShadowArgs,
    key: String,
    state: State,
    alerts: AlertManager,
}

impl Shadow {
    fn check_ratio(&self, kctx: &KCtx) -> Option<(Decimal, bool)> {
        let full = kctx.info.full.high - kctx.info.full.low;
        if full.is_zero() {
            return None;
        }

        let above = kctx.info.shadow.above;
        let below = kctx.info.shadow.below;
        let is_above = above >= below;
        let shadow_len = if is_above { above } else { below };

        let ratio = shadow_len / full;
        if ratio >= self.args.threshold {
            Some((ratio, is_above))
        } else {
            None
        }
    }

    fn event_msg(
        &self,
        ratio: Decimal,
        is_above: bool,
        colors: ColorTable,
    ) -> Msg {
        let ratio_rounded = ratio.round_dp(2);
        let (shadow_desc, shadow_color) = if is_above {
            (format!("┴{ratio_rounded}"), colors.down)
        } else {
            (format!("┬{ratio_rounded}"), colors.up)
        };

        let normal = shadow_desc.clone();

        let tty = shadow_desc.with(shadow_color).to_string();

        Msg { normal, tty }
    }
}

impl Monitor for Shadow {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![]
    }

    fn apply(&mut self, kctx: &KCtx) {
        let msg_opt = self.check_ratio(kctx).map(|(ratio, is_above)| {
            (
                kctx.info.raw.time_begin,
                self.event_msg(ratio, is_above, kctx.colors),
            )
        });

        if kctx.info.raw.finalized {
            self.state.temp.take();

            if let Some((t, m)) = msg_opt.clone() {
                self.alerts.add(t, m);
            }

            self.state.perm = msg_opt;
        } else {
            self.state.temp = msg_opt;
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
