use std::{borrow::Cow, str::FromStr};

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    config::ColorTable,
    impl_builder,
    indicator::{Calculator, ma::Ema},
    k::Trend,
    monitor::{Msg, alert::AlertManager},
    prelude::*,
};

#[derive(Debug, Clone, Copy)]
pub struct Threshold {
    pub strong: Decimal,
    pub weak: Decimal,
}

pub struct Strength {
    ma: Ema,
    threshold: Threshold,
}

impl Strength {
    fn detect(
        &self,
        next: Decimal,
        ma: Decimal,
    ) -> Option<(Decimal, Decimal, bool)> {
        if ma.is_zero() {
            return None;
        }

        let ratio = next / ma;
        if ratio >= self.threshold.strong {
            Some((next, ratio, true))
        } else if ratio <= self.threshold.weak {
            Some((next, ratio, false))
        } else {
            None
        }
    }

    fn calc(&self, next: Decimal) -> Option<(Decimal, Decimal, bool)> {
        let ma = self.ma.calc(next)?;
        self.detect(next, ma)
    }

    fn update(&mut self, next: Decimal) -> Option<(Decimal, Decimal, bool)> {
        let ma = self.ma.update(next)?;
        self.detect(next, ma)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FBQArgs {
    period: usize,
    full: (f64, f64),
    body: (f64, f64),
    qty: (f64, f64),
}

impl FromStr for FBQArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut period = 0usize;
        let mut fw = 0.0f64;
        let mut fs = 0.0f64;
        let mut bw = 0.0f64;
        let mut bs = 0.0f64;
        let mut qw = 0.0f64;
        let mut qs = 0.0f64;

        sscanf!(s, "fbq:{period},{fw}/{fs},{bw}/{bs},{qw}/{qs}").with_context(
            |_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse fbq args"),
            },
        )?;

        Ok(Self {
            period,
            full: (fw, fs),
            body: (bw, bs),
            qty: (qw, qs),
        })
    }
}

impl Args for FBQArgs {
    type Type = (usize, (f64, f64), (f64, f64), (f64, f64));
    type Target = FBQ;

    fn new(args: Self::Type) -> Self {
        Self {
            period: args.0,
            full: args.1,
            body: args.2,
            qty: args.3,
        }
    }

    fn key(&self) -> String {
        format!(
            "fbg:{},{}/{},{}/{},{}/{}",
            self.period,
            self.full.0,
            self.full.1,
            self.body.0,
            self.body.1,
            self.qty.0,
            self.qty.1
        )
    }

    fn build(self) -> Result<Self::Target> {
        let full_thres = Threshold {
            weak: Decimal::from_f64(self.full.0)
                .required("full weak threshold")?,
            strong: Decimal::from_f64(self.full.1)
                .required("full strong threshold")?,
        };

        let body_thres = Threshold {
            weak: Decimal::from_f64(self.body.0)
                .required("body weak threshold")?,
            strong: Decimal::from_f64(self.body.1)
                .required("body strong threshold")?,
        };

        let qty_thres = Threshold {
            weak: Decimal::from_f64(self.qty.0)
                .required("qty weak threshold")?,
            strong: Decimal::from_f64(self.qty.1)
                .required("qty strong threshold")?,
        };

        Ok(FBQ {
            key: self.key(),
            full: Strength {
                ma: Ema::new(self.period),
                threshold: full_thres,
            },

            body: Strength {
                ma: Ema::new(self.period),
                threshold: body_thres,
            },

            qty: Strength {
                ma: Ema::new(self.period),
                threshold: qty_thres,
            },

            trend_single_threshold: Decimal::from_f64(0.6)
                .required("trend single threshold")?,
            trend_mixed_threshold: Decimal::from_f64(0.7)
                .required("trend mixed threshold")?,

            state: Default::default(),
            alerts: Default::default(),
            prev_temp_alert_strong_flags: None,
        })
    }
}

impl_builder!(FBQBuilder: FBQArgs => FBQ);

pub struct FBQ {
    key: String,

    full: Strength,
    body: Strength,
    qty: Strength,

    trend_single_threshold: Decimal,
    trend_mixed_threshold: Decimal,

    state: State,
    alerts: AlertManager,

    prev_temp_alert_strong_flags: Option<(OffsetDateTime, u8)>,
}

impl FBQ {
    fn generate_msg(
        &self,
        trend: Trend,
        strengths: [Option<(Decimal, Decimal, bool)>; 3],
        colors: ColorTable,
    ) -> Option<(Msg, u8)> {
        const SHORTS: [&str; 3] = ["f", "b", "q"];
        let items = strengths
            .into_iter()
            .zip(SHORTS)
            .filter_map(|(s_opt, short)| s_opt.map(|s| (s, short)))
            .collect::<Vec<_>>();

        if items.is_empty() {
            return None;
        }

        let mut normal = String::new();
        let mut tty = String::new();

        let mut strong_flags = 0;
        for (n, ((_abs, ratio, dir), short)) in items.into_iter().enumerate() {
            if !normal.is_empty() {
                normal.push('|');
                tty.push('|');
            }

            if dir {
                strong_flags |= 1 << n
            }

            let ratio_rounded = ratio.round_dp(2);
            // let abs_rounded = abs.round_dp(2);
            let (flag, color) = if dir {
                ("<", colors.up)
            } else {
                (">", colors.down)
            };

            normal.push_str(&format!("{short}{flag}{ratio_rounded}"));
            tty.push_str(
                &format!("{short}{ratio_rounded}").with(color).to_string(),
            );
        }

        let trend_color = match trend {
            Trend::Up => colors.up,
            Trend::Down => colors.down,
            Trend::Unknown => colors.normal,
        };

        let trend_desc = format!("|{}", trend.as_str());
        normal.push_str(&trend_desc);
        tty.push_str(&trend_desc.with(trend_color).to_string());

        Some((Msg { normal, tty }, strong_flags))
    }
}

impl Monitor for FBQ {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![]
    }

    fn apply(&mut self, kctx: &KCtx) {
        let full = kctx.info.full.height;
        let body = kctx.info.body.height;
        let qty = kctx.info.raw.quantity;

        let strengths = if kctx.info.raw.finalized {
            [
                self.full.update(full),
                self.body.update(body),
                self.qty.update(qty),
            ]
        } else {
            [
                self.full.calc(full),
                self.body.calc(body),
                self.qty.calc(qty),
            ]
        };

        let t = kctx.info.raw.time_begin;
        let trend = kctx
            .info
            .trend(self.trend_single_threshold, self.trend_mixed_threshold);

        let msg_opt = self.generate_msg(trend, strengths, kctx.colors);

        self.state.temp.take();

        if let Some((msg, strong_flags)) = msg_opt {
            let alert_flags = (t, strong_flags);
            if self.prev_temp_alert_strong_flags != Some(alert_flags)
                && (kctx.info.raw.finalized || strong_flags > 0)
            {
                self.prev_temp_alert_strong_flags.replace(alert_flags);
                self.alerts.add(t, msg.clone());
            }

            if kctx.info.raw.finalized {
                self.state.perm.replace((t, msg));
            } else {
                self.state.temp.replace((t, msg));
            }
        } else {
            self.state.perm.take();
        };
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
