use std::{borrow::Cow, str::FromStr};

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    config::ColorTable,
    impl_builder,
    indicator::base::{BaseExtractorArgs, CalcKind, ExtractKind},
    k::Trend,
    monitor::{Msg, alert::AlertManager},
    prelude::*,
};

#[derive(Debug, Clone)]
pub struct Threshold {
    pub key: String,
    pub strong: Decimal,
    pub weak: Decimal,
}

impl Threshold {
    fn detect(
        &self,
        next: Decimal,
        ma: Decimal,
    ) -> Option<(Decimal, Decimal, bool)> {
        if ma.is_zero() {
            return None;
        }

        let ratio = next / ma;
        if ratio >= self.strong {
            Some((next, ratio, true))
        } else if ratio <= self.weak {
            Some((next, ratio, false))
        } else {
            None
        }
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
            key: BaseExtractorArgs::new((
                ExtractKind::PriceFull,
                CalcKind::Ema,
                self.period,
            ))
            .key(),
            weak: Decimal::from_f64(self.full.0)
                .required("full weak threshold")?,
            strong: Decimal::from_f64(self.full.1)
                .required("full strong threshold")?,
        };

        let body_thres = Threshold {
            key: BaseExtractorArgs::new((
                ExtractKind::PriceBody,
                CalcKind::Ema,
                self.period,
            ))
            .key(),
            weak: Decimal::from_f64(self.body.0)
                .required("body weak threshold")?,
            strong: Decimal::from_f64(self.body.1)
                .required("body strong threshold")?,
        };

        let qty_thres = Threshold {
            key: BaseExtractorArgs::new((
                ExtractKind::Qty,
                CalcKind::Ema,
                self.period,
            ))
            .key(),
            weak: Decimal::from_f64(self.qty.0)
                .required("qty weak threshold")?,
            strong: Decimal::from_f64(self.qty.1)
                .required("qty strong threshold")?,
        };

        Ok(FBQ {
            key: self.key(),

            full_thres,
            body_thres,
            qty_thres,

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

    full_thres: Threshold,

    body_thres: Threshold,

    qty_thres: Threshold,

    trend_single_threshold: Decimal,
    trend_mixed_threshold: Decimal,

    state: State,
    alerts: AlertManager,
    prev_temp_alert_strong_flags: Option<(OffsetDateTime, u8, u8)>,
}

impl FBQ {
    fn generate_msg(
        &self,
        trend: Trend,
        strengths: [Option<(Decimal, Decimal, bool)>; 3],
        colors: ColorTable,
    ) -> Option<(Msg, u8, u8)> {
        const SHORTS: [(&str, bool); 3] =
            [("F", true), ("B", true), ("Q", false)];
        let items = strengths
            .into_iter()
            .zip(SHORTS)
            .enumerate()
            .filter_map(|(n, (s_opt, (short, show_val)))| {
                s_opt.map(|s| (n, s, short, show_val))
            })
            .collect::<Vec<_>>();

        if items.is_empty() {
            return None;
        }

        let mut normal = String::new();
        let mut tty = String::new();

        let trend_color = match trend {
            Trend::Up => colors.up,
            Trend::Down => colors.down,
            Trend::Unknown => colors.normal,
        };

        let trend_desc = format!("[{}]", trend.as_str());
        normal.push_str(&trend_desc);
        tty.push_str(&trend_desc.with(trend_color).to_string());

        let mut strong_flags = 0;
        let mut weak_flags = 0;
        for (n, (abs, ratio, strong), short, show_val) in items.into_iter() {
            normal.push('|');
            tty.push('|');

            let shift = 2 - n;
            if strong {
                strong_flags |= 1 << shift
            } else {
                weak_flags |= 1 << shift
            }

            let ratio_rounded = ratio.round_dp(2);
            let abs_desc = if show_val && strong {
                let abs_rounded = abs.round_dp(2);
                format!("({abs_rounded})")
            } else {
                "".to_owned()
            };
            let (flag, color) = if strong {
                ("<", colors.up)
            } else {
                (">", colors.down)
            };

            normal.push_str(&format!("{short}{flag}{ratio_rounded}{abs_desc}"));
            tty.push_str(
                &format!("{short}{ratio_rounded}{abs_desc}")
                    .with(color)
                    .to_string(),
            );
        }

        Some((Msg { normal, tty }, strong_flags, weak_flags))
    }
}

impl Monitor for FBQ {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![
            &self.full_thres.key,
            &self.body_thres.key,
            &self.qty_thres.key,
        ]
    }

    fn apply(&mut self, kctx: &KCtx) {
        let full_next = ExtractKind::PriceFull.extractor()(&kctx.info);
        let full_ma = kctx.get_val::<Decimal>(&self.full_thres.key).copied();

        let body_next = ExtractKind::PriceBody.extractor()(&kctx.info);
        let body_ma = kctx.get_val::<Decimal>(&self.body_thres.key).copied();

        let qty_next = ExtractKind::Qty.extractor()(&kctx.info);
        let qty_ma = kctx.get_val::<Decimal>(&self.qty_thres.key).copied();

        let strengths = [
            full_ma.and_then(|ma| self.full_thres.detect(full_next, ma)),
            body_ma.and_then(|ma| self.body_thres.detect(body_next, ma)),
            qty_ma.and_then(|ma| self.qty_thres.detect(qty_next, ma)),
        ];

        let t = kctx.info.raw.time_begin;
        let trend = kctx
            .info
            .trend(self.trend_single_threshold, self.trend_mixed_threshold);

        let msg_opt = self.generate_msg(trend, strengths, kctx.colors);

        self.state.temp.take();

        if let Some((msg, strong_flags, weak_flags)) = msg_opt {
            // let alert_flags = (t, strong_flags, weak_flags);
            // if self.prev_temp_alert_strong_flags != Some(alert_flags)
            //     && (kctx.info.raw.finalized || strong_flags > 0)
            // {
            //     self.prev_temp_alert_strong_flags.replace(alert_flags);
            //     self.alerts.add(t, msg.clone());
            // }

            let mut alert_msg = None;
            // K 线已经终结
            if kctx.info.raw.finalized {
                // 信息flag和之前不一样，则需要告警
                if self.prev_temp_alert_strong_flags
                    != Some((t, strong_flags, weak_flags))
                {
                    alert_msg.replace(msg.clone());
                }

                self.state.perm.replace((t, msg));
            } else {
                // 只看能确认的部分
                const TEMP_FLAGS_MASK: u8 = 0b101;
                let masked_flags = strong_flags & TEMP_FLAGS_MASK;
                let prev_masked_flags = self
                    .prev_temp_alert_strong_flags
                    .map(|(t, sf, _wf)| (t, sf & TEMP_FLAGS_MASK));

                if strong_flags > 0
                    && prev_masked_flags != Some((t, masked_flags))
                {
                    alert_msg.replace(msg.clone());
                }

                self.state.temp.replace((t, msg));
            }

            if let Some(msg) = alert_msg {
                self.prev_temp_alert_strong_flags.replace((
                    t,
                    strong_flags,
                    weak_flags,
                ));
                self.alerts.add(t, msg);
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
