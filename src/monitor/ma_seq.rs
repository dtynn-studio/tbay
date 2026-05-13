use std::borrow::Cow;

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    impl_builder,
    indicator::base::{BaseExtractorArgs, CalcKind, ExtractKind},
    monitor::{Msg, State, alert::AlertManager},
    prelude::*,
};

#[derive(Clone)]
pub struct MaSeqArgs {
    calc_kind: CalcKind,
    periods: Vec<usize>,
    first_alert_duration: usize,
    alert_period_duration: usize,
    show_state: bool,
}

impl FromStr for MaSeqArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut calc_kind_str = String::new();
        let mut periods_str = String::new();
        let mut first_alert_duration = 0;
        let mut alert_period_duration = 0;
        let mut show_state_str = String::new();

        sscanf!(
            s,
            "ma-seq:{calc_kind_str},[{periods_str}]/{first_alert_duration}@{alert_period_duration},{show_state_str}"
        )
        .with_context(|_| ParseCtx {
            raw: s.to_owned(),
            usage: Cow::from("parse ma3 args"),
        })?;

        let calc_kind = calc_kind_str.parse()?;
        let periods = periods_str
            .split(",")
            .map(|s| s.parse())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| Error::Msg {
                reason: format!("invalid ma period: {e}").into(),
            })?;

        let period_count = periods.len();
        if period_count < 2 {
            return Err(period_count.unexpected("not enough periods"));
        }

        let show_state = show_state_str.parse().map_err(|e| Error::Msg {
            reason: format!("parse show_state: {e}").into(),
        })?;

        Ok(Self {
            calc_kind,
            periods,
            first_alert_duration,
            alert_period_duration,
            show_state,
        })
    }
}

impl Args for MaSeqArgs {
    type Type = (CalcKind, Vec<usize>, (usize, usize), bool);
    type Target = MaSeq;

    fn new(args: Self::Type) -> Self {
        Self {
            calc_kind: args.0,
            periods: args.1,
            first_alert_duration: args.2.0,
            alert_period_duration: args.2.1,
            show_state: args.3,
        }
    }

    fn key(&self) -> String {
        let periods = self
            .periods
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "ma-seq:{},[{}]/{}@{},{}",
            self.calc_kind.as_str(),
            periods,
            self.first_alert_duration,
            self.alert_period_duration,
            self.show_state,
        )
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();
        let ma_keys = self
            .periods
            .iter()
            .map(|period| {
                BaseExtractorArgs::new((
                    ExtractKind::PriceClose,
                    self.calc_kind,
                    *period,
                ))
                .key()
            })
            .collect();

        let strong_order = self.periods.clone();
        let mut weak_order = self.periods.clone();
        weak_order.reverse();

        Ok(MaSeq {
            args: self,
            key,
            ma_keys,
            strong_order,
            weak_order,
            prev: None,
            state: Default::default(),
            alert: Default::default(),
        })
    }
}

impl_builder!(MaSeqBuilder: MaSeqArgs => MaSeq);

#[derive(Clone)]
struct MaSeqRecord {
    order: Vec<usize>,
    duration: usize,
}

pub struct MaSeq {
    args: MaSeqArgs,

    key: String,
    ma_keys: Vec<String>,

    strong_order: Vec<usize>,
    weak_order: Vec<usize>,

    prev: Option<MaSeqRecord>,
    state: State,
    alert: AlertManager,
}

impl MaSeq {
    fn gen_order(&self, kctx: &KCtx) -> Option<Vec<usize>> {
        let mut order_with_price = Vec::with_capacity(self.ma_keys.len());
        for (ma_key, ma_period) in
            self.ma_keys.iter().zip(self.args.periods.iter().copied())
        {
            let ma_val = kctx.get_val::<Decimal>(ma_key).copied()?;
            order_with_price.push((ma_period, ma_val));
        }

        order_with_price.sort_by(|(_, left), (_, right)| right.cmp(left));

        Some(
            order_with_price
                .into_iter()
                .map(|(period, _val)| period)
                .collect(),
        )
    }

    fn gen_msg(&self, kctx: &KCtx, record: &MaSeqRecord) -> Msg {
        let (flag, color) = if record.order == self.strong_order {
            ('▲', kctx.colors.up)
        } else if record.order == self.weak_order {
            ('▼', kctx.colors.down)
        } else {
            ('~', kctx.colors.normal)
        };

        let mut order = String::new();
        for period in record.order.iter() {
            if !order.is_empty() {
                order.push(flag);
            }

            order.push_str(&period.to_string());
        }

        let body = format!("[{order}]@{}", record.duration);

        Msg {
            normal: body.clone(),
            tty: body.with(color).to_string(),
        }
    }
}

impl Monitor for MaSeq {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        self.ma_keys.iter().map(|k| k.as_str()).collect()
    }

    fn apply(&mut self, kctx: &KCtx) {
        if !kctx.info.raw.finalized {
            return;
        }

        let Some(order) = self.gen_order(kctx) else {
            return;
        };

        let record = if let Some(mut prev) = self.prev.take()
            && prev.order == order
        {
            prev.duration += 1;
            prev
        } else {
            MaSeqRecord { order, duration: 1 }
        };

        let t = kctx.info.t();
        let msg = self.gen_msg(kctx, &record);

        let is_first_alert = record.duration == self.args.first_alert_duration;
        let is_period_alert = self.args.alert_period_duration > 0
            && record.duration > self.args.first_alert_duration
            && (record.duration - self.args.first_alert_duration)
                .is_multiple_of(self.args.alert_period_duration);

        self.prev.replace(record);

        let should_alert = is_first_alert || is_period_alert;
        if should_alert {
            self.alert.add(t, msg.clone());
        }

        if self.args.show_state {
            self.state.perm = Some((t, msg))
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
