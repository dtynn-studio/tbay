use std::borrow::Cow;

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    impl_builder,
    indicator::base::{BaseExtractorArgs, CalcKind, ExtractKind},
    monitor::{Msg, State, alert::AlertManager},
    prelude::*,
};

#[derive(Clone, Copy)]
pub struct Ma3Args {
    calc_kind: CalcKind,
    fast: usize,
    slow: usize,
    trend: usize,
    alert_duration: usize,
}

impl FromStr for Ma3Args {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut calc_kind_str = String::new();
        let mut fast = 0;
        let mut slow = 0;
        let mut trend = 0;
        let mut alert_duration = 0;

        sscanf!(
            s,
            "ma3:{calc_kind_str},{fast},{slow},{trend}/{alert_duration}"
        )
        .with_context(|_| ParseCtx {
            raw: s.to_owned(),
            usage: Cow::from("parse ma3 args"),
        })?;

        let calc_kind = calc_kind_str.parse()?;

        Ok(Self {
            calc_kind,
            fast,
            slow,
            trend,
            alert_duration,
        })
    }
}

impl Args for Ma3Args {
    type Type = (CalcKind, usize, usize, usize, usize);
    type Target = Ma3;

    fn new(args: Self::Type) -> Self {
        Self {
            calc_kind: args.0,
            fast: args.1,
            slow: args.2,
            trend: args.3,
            alert_duration: args.4,
        }
    }

    fn key(&self) -> String {
        format!(
            "ma3:{},{},{},{}/{}",
            self.calc_kind.as_str(),
            self.fast,
            self.slow,
            self.trend,
            self.alert_duration,
        )
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();
        let fast_key = BaseExtractorArgs::new((
            ExtractKind::PriceClose,
            self.calc_kind,
            self.fast,
        ))
        .key();

        let slow_key = BaseExtractorArgs::new((
            ExtractKind::PriceClose,
            self.calc_kind,
            self.slow,
        ))
        .key();

        let trend_key = BaseExtractorArgs::new((
            ExtractKind::PriceClose,
            self.calc_kind,
            self.trend,
        ))
        .key();

        Ok(Ma3 {
            args: self,
            key,
            fast_key,
            slow_key,
            trend_key,
            prev: None,
            state: Default::default(),
            alert: Default::default(),
        })
    }
}

impl_builder!(Ma3Builder: Ma3Args => Ma3);

#[derive(Clone, Copy)]
struct Ma3Record {
    trend: Option<bool>,
    duration: usize,
}

pub struct Ma3 {
    args: Ma3Args,

    key: String,
    fast_key: String,
    slow_key: String,
    trend_key: String,

    prev: Option<Ma3Record>,
    state: State,
    alert: AlertManager,
}

impl Ma3 {
    fn get_trend(&self, kctx: &KCtx) -> Option<bool> {
        let fast = kctx.get_val::<Decimal>(&self.fast_key).copied()?;
        let slow = kctx.get_val::<Decimal>(&self.slow_key).copied()?;
        let trend = kctx.get_val::<Decimal>(&self.trend_key).copied()?;

        let fast_above = fast >= slow;
        let slow_above = slow >= trend;

        if fast_above == slow_above {
            Some(fast_above)
        } else {
            None
        }
    }

    fn gen_msg(&self, kctx: &KCtx, record: Ma3Record) -> Option<Msg> {
        let trend_dir = record.trend?;
        let (flag, color) = if trend_dir {
            ("▲", kctx.colors.up)
        } else {
            ("▼", kctx.colors.down)
        };

        let (fast, slow, trend) =
            (self.args.fast, self.args.slow, self.args.trend);

        let body = format!("{fast}{flag}{slow}{flag}{trend}");

        Some(Msg {
            normal: body.clone(),
            tty: body.with(color).to_string(),
        })
    }
}

impl Monitor for Ma3 {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.fast_key, &self.slow_key, &self.trend_key]
    }

    fn apply(&mut self, kctx: &KCtx) {
        self.state.clear();
        let trend = self.get_trend(kctx);

        let record = if let Some(mut prev) = self.prev
            && prev.trend == trend
        {
            prev.duration += 1;
            prev
        } else {
            Ma3Record { trend, duration: 1 }
        };

        let t = kctx.info.t();
        let msg_opt = self.gen_msg(kctx, record);

        if kctx.info.raw.finalized {
            self.prev.replace(record);

            if trend.is_some()
                && record.duration == self.args.alert_duration
                && let Some(m) = msg_opt.as_ref()
            {
                self.alert.add(t, m.clone());
            }

            self.state.perm = msg_opt.map(|m| (t, m))
        } else {
            self.state.temp = msg_opt.map(|m| (t, m))
        }
    }

    fn state(&self) -> &State {
        &self.state
    }

    fn take_alerts(&mut self) -> Vec<(OffsetDateTime, Msg)> {
        self.alert.take()
    }

    fn is_once(&self) -> bool {
        false
    }

    fn terminated(&self) -> bool {
        false
    }
}
