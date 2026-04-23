//! 实现 Burst Monitor 用以检测突发的动力/阻力
//! [参考](https://www.doubao.com/thread/wf1c7cac2ffe5c0b6)
//! 核心：效率计算： V / (W_body * body_height + W_shadow * shadow_height)
//! 简化：实体全重为 1，影线全重交给参数定义
//! 格式：burst:{ma_period},{shadow_weight},{weak_threshold}~{strong_threshold},{alert_periods}

use std::{borrow::Cow, fmt::Write};

use crossterm::style::{Color, Stylize};
use scanf::sscanf;

use crate::{
    common::impl_builder,
    config::ColorTable,
    indicator::{Calculator, ma::Sma},
    monitor::alert::AlertManager,
    prelude::*,
};

const ALERT_POINT_TEMP: &str = "temp";
const ALERT_POINT_PERM: &str = "perm";

#[derive(Clone)]
pub struct BurstArgs {
    ma_period: usize,
    shadow_weight: f64,
    weak_threshold: f64,
    strong_threshold: f64,
    alert_point: String,
    alert_periods: usize,
}

impl FromStr for BurstArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ma_period = 0usize;
        let mut shadow_weight = 0.0f64;
        let mut weak_threshold = 0.0f64;
        let mut strong_threshold = 0.0f64;
        let mut alert_point = String::new();
        let mut alert_periods = 0usize;

        sscanf!(s, "burst:{ma_period},{shadow_weight},{weak_threshold}~{strong_threshold},{alert_point}@{alert_periods}")
            .with_context(|_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse burst args"),
            })?;

        Ok(Self {
            ma_period,
            shadow_weight,
            weak_threshold,
            strong_threshold,
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
            weak_threshold: args.2,
            strong_threshold: args.3,
            alert_point: args.4,
            alert_periods: args.5,
        }
    }

    fn key(&self) -> String {
        format!(
            "burst:{},{},{}~{},{}@{}",
            self.ma_period,
            self.shadow_weight,
            self.weak_threshold,
            self.strong_threshold,
            self.alert_point,
            self.alert_periods,
        )
    }

    fn build(self) -> Result<Self::Target> {
        let alert_for_perm = match self.alert_point.as_str() {
            ALERT_POINT_PERM => true,
            ALERT_POINT_TEMP => false,
            other => return Err(other.unexpected("alert point")),
        };
        let key = self.key();

        let shadow_weight =
            Decimal::from_f64(self.shadow_weight).required("shadow weight")?;

        let weak_threshold = Decimal::from_f64(self.weak_threshold)
            .required("weak threshold")?;

        let strong_threshold = Decimal::from_f64(self.strong_threshold)
            .required("strong threshold")?;

        let up_ma = Sma::new(self.ma_period);
        let down_ma = Sma::new(self.ma_period);

        Ok(Burst {
            args: self,
            key,
            up_ma,
            down_ma,
            alert_for_perm,
            shadow_weight,
            weak_threshold,
            strong_threshold,
            prev: None,
            prev_alert_t: None,
            state: Default::default(),
            alert: Default::default(),
        })
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Strength {
    Strong, // (strong_threshold, )
    Normal, // [weak_threshold, strong_threshold]
    Weak,   // (, weak_threshold)
}

impl Strength {
    pub fn flag(self, colors: ColorTable) -> (&'static str, Color) {
        match self {
            Self::Strong => ("<", colors.up),
            Self::Normal => ("~", colors.normal),
            Self::Weak => (">", colors.down),
        }
    }
}

#[derive(Clone, Copy)]
pub struct EffortItem {
    // val: Decimal,
    // ma: Decimal,
    rate: Decimal,
    strength: Strength,
}

#[derive(Clone, Copy)]
pub struct Effort {
    up: (EffortItem, usize),
    down: (EffortItem, usize),
}

impl_builder!(BurstBuilder: BurstArgs => Burst);

pub struct Burst {
    args: BurstArgs,
    key: String,
    alert_for_perm: bool,

    up_ma: Sma,
    down_ma: Sma,

    shadow_weight: Decimal,
    weak_threshold: Decimal,
    strong_threshold: Decimal,

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

        let up_effort = kctx.info.raw.quantity / up_height;
        let down_effort = kctx.info.raw.quantity / down_height;

        (down_effort, up_effort)
    }

    fn calc(&self, kctx: &KCtx) -> Option<Effort> {
        let (down_effort, up_effort) = self.calc_efforts(kctx);
        let down_ma = self.down_ma.calc(down_effort)?;
        let up_ma = self.up_ma.calc(up_effort)?;

        let down_effort_item = self.generate_effort_item(down_effort, down_ma);
        let up_effort_item = self.generate_effort_item(up_effort, up_ma);

        let effort = self.generate_effort(down_effort_item, up_effort_item);
        Some(effort)
    }

    fn update(&mut self, kctx: &KCtx) -> Option<Effort> {
        let (down_effort, up_effort) = self.calc_efforts(kctx);
        let down_ma = self.down_ma.update(down_effort)?;
        let up_ma = self.up_ma.update(up_effort)?;

        let down_effort_item = self.generate_effort_item(down_effort, down_ma);
        let up_effort_item = self.generate_effort_item(up_effort, up_ma);

        let effort = self.generate_effort(down_effort_item, up_effort_item);
        Some(effort)
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
            "[⬆]",
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
            "[⬇]",
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
        prefix: &str,
        (next, next_periods): &(EffortItem, usize),
        for_alert: bool,
    ) {
        // 常规情形，或持续时长不满足
        if next.strength == Strength::Normal
            || (for_alert && *next_periods != self.args.alert_periods)
        {
            return;
        }

        let rate = next.rate.round_dp(2);

        let (flag, flag_color) = next.strength.flag(kctx.colors);

        let info = format!("{flag}{rate}");

        _ = write!(normal_msg, "{prefix}:{info}[{next_periods}]");
        _ = write!(
            tty_msg,
            "{prefix}:{}[{next_periods}]",
            info.with(flag_color),
        );
    }

    fn generate_effort(
        &self,
        down_effort_item: EffortItem,
        up_effort_item: EffortItem,
    ) -> Effort {
        let (down_periods, up_periods) = if let Some(prev) = self.prev.as_ref()
        {
            let down_periods =
                if prev.down.0.strength == down_effort_item.strength {
                    prev.down.1 + 1
                } else {
                    1
                };

            let up_periods = if prev.up.0.strength == up_effort_item.strength {
                prev.up.1 + 1
            } else {
                1
            };

            (down_periods, up_periods)
        } else {
            (1, 1)
        };

        Effort {
            up: (up_effort_item, up_periods),
            down: (down_effort_item, down_periods),
        }
    }

    fn generate_effort_item(&self, val: Decimal, ma: Decimal) -> EffortItem {
        let rate = val / ma;
        let strength = if rate > self.strong_threshold {
            Strength::Strong
        } else if rate < self.weak_threshold {
            Strength::Weak
        } else {
            Strength::Normal
        };

        EffortItem {
            // val,
            // ma,
            rate,
            strength,
        }
    }
}

impl Monitor for Burst {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![]
    }

    fn apply(&mut self, kctx: &KCtx) {
        let t = kctx.info.t();

        let (allow_alert, effort) = if kctx.info.raw.finalized {
            (self.alert_for_perm, self.update(kctx))
        } else {
            (
                !self.alert_for_perm && self.prev_alert_t != Some(t),
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

        if kctx.info.raw.finalized {
            self.prev = effort;
            self.state.perm = state_msg;
        } else {
            self.state.temp = state_msg;
        };
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
