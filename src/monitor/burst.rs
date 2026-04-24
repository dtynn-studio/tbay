//! 实现 Burst Monitor 用以检测突发的动力/阻力
//! [参考](https://www.doubao.com/thread/wf1c7cac2ffe5c0b6)
//! 核心：效率计算： Quantity / (W_body * body_height + W_shadow * shadow_height)
//! 简化：实体全重为 1，影线全重交给参数定义
//! 格式：burst:{ma_period},{shadow_weight},{strong_threshold},{qty_threshold},{alert_point}@{alert_periods}

use std::{borrow::Cow, fmt::Write};

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    common::impl_builder,
    indicator::{
        Calculator,
        base::{BaseExtractorArgs, CalcKind, ExtractKind},
        ma::Sma,
    },
    monitor::alert::AlertManager,
    prelude::*,
};

const ALERT_POINT_TEMP: &str = "temp";
const ALERT_POINT_PERM: &str = "perm";

#[derive(Clone)]
pub struct BurstArgs {
    ma_period: usize,
    shadow_weight: f64,
    strong_threshold: f64,
    qty_threshold: f64,
    alert_point: String,
    alert_periods: usize,
}

impl FromStr for BurstArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ma_period = 0usize;
        let mut shadow_weight = 0.0f64;
        let mut strong_threshold = 0.0f64;
        let mut qty_threshold = 0.0f64;
        let mut alert_point = String::new();
        let mut alert_periods = 0usize;

        sscanf!(s, "burst:{ma_period},{shadow_weight},{strong_threshold},{qty_threshold},{alert_point}@{alert_periods}")
            .with_context(|_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse burst args"),
            })?;

        Ok(Self {
            ma_period,
            shadow_weight,
            strong_threshold,
            qty_threshold,
            alert_point,
            alert_periods,
        })
    }
}

impl Args for BurstArgs {
    type Type = (usize, f64, f64, f64, String, usize);

    type Target = Burst;

    fn new(args: Self::Type) -> Self {
        Self {
            ma_period: args.0,
            shadow_weight: args.1,
            strong_threshold: args.2,
            qty_threshold: args.3,
            alert_point: args.4,
            alert_periods: args.5,
        }
    }

    fn key(&self) -> String {
        format!(
            "burst:{},{},{},{},{}@{}",
            self.ma_period,
            self.shadow_weight,
            self.strong_threshold,
            self.qty_threshold,
            self.alert_point,
            self.alert_periods,
        )
    }

    fn build(self) -> Result<Self::Target> {
        let is_perm = match self.alert_point.as_str() {
            ALERT_POINT_PERM => true,
            ALERT_POINT_TEMP => false,
            other => return Err(other.unexpected("alert point")),
        };
        let key = self.key();

        let shadow_weight =
            Decimal::from_f64(self.shadow_weight).required("shadow weight")?;

        let strong_threshold = Decimal::from_f64(self.strong_threshold)
            .required("strong threshold")?;

        let qty_threshold =
            Decimal::from_f64(self.qty_threshold).required("qty threshold")?;
        let qty_ma_key = BaseExtractorArgs::new((
            ExtractKind::Qty,
            CalcKind::Sma,
            self.ma_period,
        ))
        .key();

        let trend_single_threshold =
            Decimal::from_f64(0.6).required("trend single threshold")?;
        let trend_mixed_threshold =
            Decimal::from_f64(0.7).required("trend mixed threshold")?;

        let up_ma = Sma::new(self.ma_period);
        let down_ma = Sma::new(self.ma_period);

        Ok(Burst {
            args: self,
            key,
            up_ma,
            down_ma,
            is_perm,
            shadow_weight,
            strong_threshold,
            qty_ma_key,
            qty_threshold,
            trend_single_threshold,
            trend_mixed_threshold,
            prev: None,
            prev_alert_t: None,
            state: Default::default(),
            alert: Default::default(),
        })
    }
}

#[derive(Clone, Copy)]
pub struct Effort {
    up: (Decimal, usize, bool),
    down: (Decimal, usize, bool),
}

impl_builder!(BurstBuilder: BurstArgs => Burst);

pub struct Burst {
    args: BurstArgs,
    key: String,
    is_perm: bool,

    up_ma: Sma,
    down_ma: Sma,

    shadow_weight: Decimal,
    strong_threshold: Decimal,

    qty_ma_key: String,
    qty_threshold: Decimal,

    trend_single_threshold: Decimal,
    trend_mixed_threshold: Decimal,

    prev: Option<Effort>,
    prev_alert_t: Option<OffsetDateTime>,

    state: State,
    alert: AlertManager,
}

impl Burst {
    fn calc_efforts(&self, kctx: &KCtx) -> (Decimal, Decimal) {
        let mut up_height = kctx.info.shadow.below * self.shadow_weight;
        let mut down_height = kctx.info.shadow.above * self.shadow_weight;

        match kctx.info.direction {
            Some(true) => {
                up_height += kctx.info.body.height;
            }

            Some(false) => {
                down_height += kctx.info.body.height;
            }

            None => {}
        }

        let up_effort = if up_height > Decimal::ZERO {
            kctx.info.raw.quantity / up_height
        } else {
            Decimal::ZERO
        };

        let down_effort = if down_height > Decimal::ZERO {
            kctx.info.raw.quantity / down_height
        } else {
            Decimal::ZERO
        };

        (down_effort, up_effort)
    }

    fn calc(&self, kctx: &KCtx) -> Option<Effort> {
        let (down_effort, up_effort) = self.calc_efforts(kctx);
        let down_ma = self.down_ma.calc(down_effort)?;
        let up_ma = self.up_ma.calc(up_effort)?;

        let effort =
            self.generate_effort((down_effort, down_ma), (up_effort, up_ma));
        Some(effort)
    }

    fn update(&mut self, kctx: &KCtx) -> Option<Effort> {
        let (down_effort, up_effort) = self.calc_efforts(kctx);
        let down_ma = self.down_ma.update(down_effort)?;
        let up_ma = self.up_ma.update(up_effort)?;

        let effort =
            self.generate_effort((down_effort, down_ma), (up_effort, up_ma));
        Some(effort)
    }

    fn generate_effort(
        &self,
        (down_effort, down_ma): (Decimal, Decimal),
        (up_effort, up_ma): (Decimal, Decimal),
    ) -> Effort {
        let down_effort_rate = if down_ma > Decimal::ZERO {
            down_effort / down_ma
        } else {
            Decimal::ZERO
        };

        let up_effort_rate = if up_ma > Decimal::ZERO {
            up_effort / up_ma
        } else {
            Decimal::ZERO
        };

        let down_burst = down_effort_rate > self.strong_threshold;
        let up_burst = up_effort_rate > self.strong_threshold;

        let (down_periods, up_periods) = if let Some(prev) = self.prev.as_ref()
        {
            (
                if down_burst == prev.down.2 {
                    prev.down.1 + 1
                } else {
                    1
                },
                if up_burst == prev.up.2 {
                    prev.up.1 + 1
                } else {
                    1
                },
            )
        } else {
            (1, 1)
        };

        Effort {
            up: (up_effort_rate, up_periods, up_burst),
            down: (down_effort_rate, down_periods, down_burst),
        }
    }

    fn generate_msg(
        &self,
        kctx: &KCtx,
        effort: &Effort,
        for_alert: bool,
    ) -> Option<Msg> {
        let mut normal = String::new();
        let mut tty = String::new();

        self.generate_msg_for_item(
            kctx,
            &mut normal,
            &mut tty,
            false,
            &effort.up,
            for_alert,
        );

        if !normal.is_empty() {
            normal.push(' ');
            tty.push(' ');
        }

        self.generate_msg_for_item(
            kctx,
            &mut normal,
            &mut tty,
            true,
            &effort.down,
            for_alert,
        );

        if !normal.is_empty() {
            Some(Msg { normal, tty })
        } else {
            None
        }
    }

    fn generate_msg_for_item(
        &self,
        kctx: &KCtx,
        normal_msg: &mut String,
        tty_msg: &mut String,
        direction: bool,
        (rate, periods, is_busrt): &(Decimal, usize, bool),
        for_alert: bool,
    ) {
        // 常规情形，或持续时长不满足
        if !is_busrt || (for_alert && *periods != self.args.alert_periods) {
            return;
        }

        let (direction_flag, direction_color) = if direction {
            ("↗", kctx.colors.up)
        } else {
            ("↘", kctx.colors.down)
        };

        let rate = rate.round_dp(2);

        let info = format!("{direction_flag}{rate}");
        let trend = kctx
            .info
            .trend(self.trend_single_threshold, self.trend_mixed_threshold);

        _ = write!(normal_msg, "🚀:{info}@{periods}[{}]", trend.as_str());
        _ = write!(
            tty_msg,
            "🚀:{}@{periods}[{}]",
            info.with(direction_color),
            trend.as_str()
        );
    }
}

impl Monitor for Burst {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.qty_ma_key]
    }

    fn apply(&mut self, kctx: &KCtx) {
        let qty_next = ExtractKind::Qty.extractor()(&kctx.info);
        let qty_ma = kctx.get_val::<Decimal>(&self.qty_ma_key).copied();
        let qty_ok = qty_ma
            .map(|qma| {
                qma > Decimal::ZERO && (qty_next / qma) > self.qty_threshold
            })
            .unwrap_or(false);

        if !qty_ok {
            self.prev.take();
            self.state.temp.take();
            self.state.perm.take();
            return;
        }

        let t = kctx.info.t();

        let (allow_alert, effort) = if kctx.info.raw.finalized {
            (self.is_perm, self.update(kctx))
        } else {
            (
                !self.is_perm && self.prev_alert_t != Some(t),
                self.calc(kctx),
            )
        };

        if let Some(msg) = allow_alert.then_some(()).and_then(|_| {
            effort
                .as_ref()
                .and_then(|e| self.generate_msg(kctx, e, true))
        }) {
            self.prev_alert_t.replace(t);
            self.alert.add(t, msg);
        }

        let state_msg = effort
            .as_ref()
            .and_then(|e| self.generate_msg(kctx, e, false))
            .map(|m| (t, m));

        let dest = if kctx.info.raw.finalized {
            self.prev = effort;
            self.state.temp.take();
            &mut self.state.perm
        } else {
            &mut self.state.temp
        };

        if self.is_perm {
            *dest = state_msg;
        }
    }

    fn state(&self) -> &State {
        &self.state
    }

    fn take_alerts(&mut self) -> Vec<(OffsetDateTime, Msg)> {
        self.alert.take()
    }

    fn terminated(&self) -> bool {
        false
    }

    fn is_once(&self) -> bool {
        false
    }
}
