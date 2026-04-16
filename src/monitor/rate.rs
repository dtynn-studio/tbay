use std::{borrow::Cow, str::FromStr};

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    config::ColorTable,
    impl_builder,
    indicator::base::{BaseExtractorArgs, CalcKind, ExtractKind},
    monitor::{Msg, alert::AlertManager},
    prelude::*,
};

#[derive(Debug, Clone, Copy)]
pub enum RateMode {
    Abs,
    Dif,
}

impl RateMode {
    pub const ABS_STR: &str = "abs";
    pub const DIF_STR: &str = "dif";

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Abs => Self::ABS_STR,
            Self::Dif => Self::DIF_STR,
        }
    }
}

impl FromStr for RateMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::ABS_STR => Ok(Self::Abs),
            Self::DIF_STR => Ok(Self::Dif),
            other => Err(other.unexpected("parse rate mode")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Gt,
    Lt,
}

impl Op {
    pub const GT_STR: &str = ">";
    pub const LT_STR: &str = "<";

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Gt => Self::GT_STR,
            Self::Lt => Self::LT_STR,
        }
    }

    pub fn check(&self, rate: Decimal, threshold: Decimal) -> bool {
        match self {
            Self::Gt => rate > threshold,
            Self::Lt => rate < threshold,
        }
    }
}

impl FromStr for Op {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::GT_STR => Ok(Self::Gt),
            Self::LT_STR => Ok(Self::Lt),
            other => Err(other.unexpected("parse op")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateArgs {
    pub val_kind: ExtractKind,
    pub calc_kind: CalcKind,
    pub period: u32,
    pub mode: RateMode,
    pub op: Op,
    pub threshold: f64,
}

impl FromStr for RateArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut val_kind_str = String::new();
        let mut calc_kind_str = String::new();
        let mut period = 0u32;
        let mut mode_str = String::new();
        let mut op_str = String::new();
        let mut threshold = 0.0f64;

        sscanf!(s, "rate:{val_kind_str},{calc_kind_str},{period},{mode_str},{op_str},{threshold}").with_context(
            |_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse rate args"),
            },
        )?;

        let val_kind = val_kind_str.parse()?;
        let calc_kind = calc_kind_str.parse()?;
        let mode = mode_str.parse()?;
        let op = op_str.parse()?;

        Ok(Self {
            val_kind,
            calc_kind,
            period,
            mode,
            op,
            threshold,
        })
    }
}

impl Args for RateArgs {
    type Type = (ExtractKind, CalcKind, u32, RateMode, Op, f64);
    type Target = Rate;

    fn new(args: Self::Type) -> Self {
        Self {
            val_kind: args.0,
            calc_kind: args.1,
            period: args.2,
            mode: args.3,
            op: args.4,
            threshold: args.5,
        }
    }

    fn key(&self) -> String {
        format!(
            "rate:{},{},{},{},{},{}",
            self.val_kind.as_str(),
            self.calc_kind.as_str(),
            self.period,
            self.mode.as_str(),
            self.op.as_str(),
            self.threshold
        )
    }

    fn build(self) -> Result<Self::Target> {
        let threshold =
            Decimal::from_f64(self.threshold).required("rate threshold")?;

        let base_args = BaseExtractorArgs::new((
            self.val_kind,
            self.calc_kind,
            self.period as usize,
        ));
        let base_key = base_args.key();
        let key = self.key();

        Ok(Rate {
            args: self,
            threshold,
            hundred: Decimal::from(100),
            key,
            base_key,
            state: Default::default(),
            alerts: Default::default(),
            temp_t: None,
            perm_t: None,
        })
    }
}

impl_builder!(RateBuilder: RateArgs => Rate);

pub struct Rate {
    args: RateArgs,
    threshold: Decimal,
    hundred: Decimal,
    key: String,
    base_key: String,
    state: State,
    alerts: AlertManager,
    temp_t: Option<OffsetDateTime>,
    perm_t: Option<OffsetDateTime>,
}

impl Rate {
    fn val(&self, kctx: &KCtx) -> Decimal {
        self.args.val_kind.extractor()(&kctx.info)
    }

    fn calc(&self, kctx: &KCtx) -> Option<Msg> {
        let val = self.val(kctx);
        let base: Decimal = *kctx.get_val::<Decimal>(&self.base_key)?;

        if base.is_zero() {
            return None;
        }

        let rate = match self.args.mode {
            RateMode::Abs => val / base,
            RateMode::Dif => (val - base).abs() / base,
        };

        if !self.args.op.check(rate, self.threshold) {
            return None;
        }

        let msg = self.format_msg(rate, kctx.colors);
        Some(msg)
    }

    fn format_msg(&self, rate: Decimal, colors: ColorTable) -> Msg {
        let (rate_str, color) = match self.args.mode {
            RateMode::Abs => (format!("{}x", rate.round_dp(2)), colors.normal),
            RateMode::Dif => {
                let pct = rate * self.hundred;
                let (sign, sign_color) = if rate.is_sign_negative() {
                    ("-", colors.down)
                } else {
                    ("+", colors.up)
                };
                (format!("{}{}%", sign, pct.round_dp(2)), sign_color)
            }
        };

        let normal = format!(
            "{}/{}{}:{}",
            self.args.val_kind.as_str_short(),
            self.args.calc_kind.as_str_short(),
            self.args.period,
            rate_str,
        );

        let tty = format!(
            "{}/{}{}:{}",
            self.args.val_kind.as_str_short(),
            self.args.calc_kind.as_str_short(),
            self.args.period,
            rate_str.with(color),
        );

        Msg { normal, tty }
    }
}

impl Monitor for Rate {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.base_key]
    }

    fn apply(&mut self, kctx: &KCtx) {
        let msg_opt = self.calc(kctx);
        let t = kctx.info.raw.time_begin;

        let (target, prev_t, should_update) = if kctx.info.raw.finalized {
            self.state.temp.take();
            (
                &mut self.state.perm,
                &mut self.perm_t,
                self.args.op == Op::Lt,
            )
        } else {
            (
                &mut self.state.temp,
                &mut self.temp_t,
                self.args.op == Op::Gt,
            )
        };

        // 无论如何，更新target
        if should_update {
            *target = msg_opt.clone().map(|msg| (t, msg));
        }

        // 当阈值条件符合
        //      (finalized && op == Lt) || (!finalized && op == Gt)
        // 且为初次更新（时间匹配）时，添加告警
        if let Some(msg) = msg_opt
            && should_update
            && Some(t) != *prev_t
        {
            prev_t.replace(t);
            self.alerts.add(t, msg);
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
