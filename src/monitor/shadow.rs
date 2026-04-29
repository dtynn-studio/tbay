use std::borrow::Cow;

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    config::ColorTable,
    impl_builder,
    indicator::base::{BaseExtractorArgs, CalcKind, ExtractKind},
    k::StrengthChecker,
    monitor::{Msg, alert::AlertManager},
    prelude::*,
};

#[derive(Debug, Clone, Copy)]
pub struct ShadowArgs {
    threshold: f64,
    full_thres: f64,
}

impl FromStr for ShadowArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut threshold = 0.0f64;
        let mut full_thres = 0.0f64;

        if sscanf!(s, "shadow:{threshold}").is_err() {
            sscanf!(s, "shadow:{threshold},{full_thres}").with_context(
                |_| ParseCtx {
                    raw: s.to_owned(),
                    usage: Cow::from("parse shadow args"),
                },
            )?;
        }

        Ok(Self {
            threshold,
            full_thres,
        })
    }
}

impl Args for ShadowArgs {
    type Type = (f64, f64);
    type Target = Shadow;

    fn new(args: Self::Type) -> Self {
        Self {
            threshold: args.0,
            full_thres: args.1,
        }
    }

    fn key(&self) -> String {
        if self.full_thres > 0.0 {
            format!("shadow:{},{}", self.threshold, self.full_thres)
        } else {
            format!("shadow:{}", self.threshold)
        }
    }

    fn build(self) -> Result<Self::Target> {
        let threshold =
            Decimal::from_f64(self.threshold).required("shadow threshold")?;

        let key = self.key();

        let checker = if self.full_thres > 0.0 {
            let val_kind = ExtractKind::PriceFull;
            let key =
                BaseExtractorArgs::new((val_kind, CalcKind::Sma, 20)).key();
            let thres = Decimal::from_f64(self.full_thres)
                .required("full threshold")?;
            Some(StrengthChecker::new(val_kind, key, thres))
        } else {
            None
        };

        Ok(Shadow {
            _args: self,
            key,
            threshold,
            checker,
            state: Default::default(),
            alerts: Default::default(),
        })
    }
}

impl_builder!(ShadowBuilder: ShadowArgs => Shadow);

pub struct Shadow {
    _args: ShadowArgs,
    key: String,
    threshold: Decimal,
    checker: Option<StrengthChecker>,
    state: State,
    alerts: AlertManager,
}

impl Shadow {
    fn check_ratio(&self, kctx: &KCtx) -> Option<(Decimal, bool)> {
        let full = kctx.info.full.height;
        if full.is_zero() {
            return None;
        }

        let above = kctx.info.shadow.above;
        let below = kctx.info.shadow.below;
        let is_above = above >= below;
        let shadow_len = if is_above { above } else { below };

        let ratio = shadow_len / full;

        if ratio >= self.threshold {
            Some((ratio, is_above))
        } else {
            None
        }
    }

    fn event_msg(
        &self,
        val: Decimal,
        ratio: Decimal,
        is_above: bool,
        colors: ColorTable,
    ) -> Msg {
        let val_rounded = val.round_dp(2);
        let ratio_rounded = ratio.round_dp(2);
        let (shadow_desc, shadow_color) = if is_above {
            (format!("┴{ratio_rounded}({val_rounded})"), colors.down)
        } else {
            (format!("┬{ratio_rounded}({val_rounded})"), colors.up)
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
        if let Some(c) = self.checker.as_ref() {
            vec![&c.key]
        } else {
            vec![]
        }
    }

    fn apply(&mut self, kctx: &KCtx) {
        let checked =
            self.checker.as_ref().map(|c| c.check(kctx)).unwrap_or(true);

        let msg_opt = self.check_ratio(kctx).map(|(ratio, is_above)| {
            (
                kctx.info.raw.time_begin,
                self.event_msg(
                    kctx.info.full.height,
                    ratio,
                    is_above,
                    kctx.colors,
                ),
            )
        });

        if kctx.info.raw.finalized {
            self.state.temp.take();

            if let Some((t, m)) = msg_opt.clone()
                && checked
            {
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

    fn is_once(&self) -> bool {
        false
    }
}
