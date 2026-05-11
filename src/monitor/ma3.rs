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
    first_alert_duration: usize,
    alert_period_duration: usize,
}

impl FromStr for Ma3Args {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut calc_kind_str = String::new();
        let mut fast = 0;
        let mut slow = 0;
        let mut trend = 0;
        let mut first_alert_duration = 0;
        let mut alert_period_duration = 0;

        sscanf!(
            s,
            "ma3:{calc_kind_str},{fast},{slow},{trend}/{first_alert_duration}@{alert_period_duration}"
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
            first_alert_duration,
            alert_period_duration,
        })
    }
}

impl Args for Ma3Args {
    type Type = (CalcKind, (usize, usize, usize), (usize, usize));
    type Target = Ma3;

    fn new(args: Self::Type) -> Self {
        Self {
            calc_kind: args.0,
            fast: args.1.0,
            slow: args.1.1,
            trend: args.1.2,
            first_alert_duration: args.2.0,
            alert_period_duration: args.2.1,
        }
    }

    fn key(&self) -> String {
        format!(
            "ma3:{},{},{},{}/{}@{}",
            self.calc_kind.as_str(),
            self.fast,
            self.slow,
            self.trend,
            self.first_alert_duration,
            self.alert_period_duration,
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
            strong_order: [self.fast, self.slow, self.trend],
            weak_order: [self.trend, self.slow, self.fast],
            prev: None,
            state: Default::default(),
            alert: Default::default(),
        })
    }
}

impl_builder!(Ma3Builder: Ma3Args => Ma3);

#[derive(Clone, Copy)]
struct Ma3Record {
    order: [usize; 3],
    duration: usize,
}

pub struct Ma3 {
    args: Ma3Args,

    key: String,
    fast_key: String,
    slow_key: String,
    trend_key: String,

    strong_order: [usize; 3],
    weak_order: [usize; 3],

    prev: Option<Ma3Record>,
    state: State,
    alert: AlertManager,
}

impl Ma3 {
    fn gen_order(&self, kctx: &KCtx) -> Option<[usize; 3]> {
        let fast = kctx.get_val::<Decimal>(&self.fast_key).copied()?;
        let slow = kctx.get_val::<Decimal>(&self.slow_key).copied()?;
        let trend = kctx.get_val::<Decimal>(&self.trend_key).copied()?;

        let mut order_with_price = [
            (self.args.fast, fast),
            (self.args.slow, slow),
            (self.args.trend, trend),
        ];

        order_with_price.sort_by(|(_, left), (_, right)| right.cmp(left));

        Some(std::array::from_fn(|i| order_with_price[i].0))
    }

    fn gen_msg(&self, kctx: &KCtx, record: Ma3Record) -> Msg {
        let (flag, color) = if record.order == self.strong_order {
            ("▲", kctx.colors.up)
        } else if record.order == self.weak_order {
            ("▼", kctx.colors.down)
        } else {
            ("~", kctx.colors.normal)
        };

        let body = format!(
            "[{}{flag}{}{flag}{}]@{}",
            record.order[0], record.order[1], record.order[2], record.duration
        );

        Msg {
            normal: body.clone(),
            tty: body.with(color).to_string(),
        }
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
        if !kctx.info.raw.finalized {
            return;
        }

        let Some(order) = self.gen_order(kctx) else {
            return;
        };

        let record = if let Some(mut prev) = self.prev
            && prev.order == order
        {
            prev.duration += 1;
            prev
        } else {
            Ma3Record { order, duration: 1 }
        };

        let t = kctx.info.t();
        let msg = self.gen_msg(kctx, record);

        self.prev.replace(record);

        let is_first_alert = record.duration == self.args.first_alert_duration;
        let is_period_alert = self.args.alert_period_duration > 0
            && record.duration > self.args.first_alert_duration
            && (record.duration - self.args.first_alert_duration)
                .is_multiple_of(self.args.alert_period_duration);

        let should_alert = is_first_alert || is_period_alert;
        if should_alert {
            self.alert.add(t, msg.clone());
        }

        self.state.perm = Some((t, msg))
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
