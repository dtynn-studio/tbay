use std::{borrow::Cow, collections::VecDeque};

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

#[derive(Clone)]
struct TimeTicks {
    ts: VecDeque<OffsetDateTime>,
    cap: usize,
}

impl TimeTicks {
    fn new(cap: usize) -> Self {
        Self {
            ts: Default::default(),
            cap,
        }
    }

    fn add(&mut self, t: OffsetDateTime) -> bool {
        if self.ts.back() == Some(&t) {
            return false;
        }

        if self.ts.len() >= self.cap {
            self.ts.pop_front();
        }

        self.ts.push_back(t);
        true
    }

    fn is_recent(&self, t: OffsetDateTime) -> bool {
        self.ts.iter().any(|pt| pt == &t)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TouchArgs {
    val_kind: ExtractKind,
    calc_kind: CalcKind,
    ma: usize,
    full_threshold: f64,
}

impl FromStr for TouchArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut val_kind_str = String::new();
        let mut calc_kind_str = String::new();
        let mut ma = 0usize;
        let mut full_threshold = 0.0;

        if sscanf!(
            s,
            "touch:{val_kind_str},{calc_kind_str},{ma},{full_threshold}"
        )
        .is_err()
        {
            sscanf!(s, "touch:{val_kind_str},{calc_kind_str},{ma}")
                .with_context(|_| ParseCtx {
                    raw: s.to_owned(),
                    usage: Cow::from("parse touch args"),
                })?;
        }

        let val_kind = val_kind_str.parse()?;
        let calc_kind = calc_kind_str.parse()?;

        Ok(Self {
            val_kind,
            calc_kind,
            ma,
            full_threshold,
        })
    }
}

impl Args for TouchArgs {
    type Type = (ExtractKind, CalcKind, usize, f64);
    type Target = Touch;

    fn new(args: Self::Type) -> Self {
        Self {
            val_kind: args.0,
            calc_kind: args.1,
            ma: args.2,
            full_threshold: args.3,
        }
    }

    fn key(&self) -> String {
        if self.full_threshold > 0.0 {
            format!(
                "touch:{},{},{},{}",
                self.val_kind.as_str(),
                self.calc_kind.as_str(),
                self.ma,
                self.full_threshold,
            )
        } else {
            format!(
                "touch:{},{},{}",
                self.val_kind.as_str(),
                self.calc_kind.as_str(),
                self.ma
            )
        }
    }

    fn build(self) -> Result<Self::Target> {
        if !matches!(self.val_kind, ExtractKind::PriceClose | ExtractKind::Qty)
        {
            return Err(self.val_kind.unexpected("val kind for touch"));
        }

        let ma_args =
            BaseExtractorArgs::new((self.val_kind, self.calc_kind, self.ma));
        let ma_key = ma_args.key();
        let key = self.key();

        let checker = if self.full_threshold > 0.0 {
            let thres = Decimal::from_f64(self.full_threshold)
                .required("full threshold")?;
            let key = BaseExtractorArgs::new((
                ExtractKind::PriceFull,
                CalcKind::Ema,
                20,
            ))
            .key();
            Some(StrengthChecker::new(ExtractKind::PriceFull, key, thres))
        } else {
            None
        };

        Ok(Touch {
            args: self,
            key,
            ma_key,
            checker,
            prev_touched: None,
            state: Default::default(),
            alerts: Default::default(),
            prev_alert_time: None,
            ticks: TimeTicks::new(2),
        })
    }
}

impl_builder!(TouchBuilder: TouchArgs => Touch);

pub struct Touch {
    args: TouchArgs,
    key: String,
    ma_key: String,
    checker: Option<StrengthChecker>,
    prev_touched: Option<bool>,
    state: State,
    alerts: AlertManager,
    prev_alert_time: Option<OffsetDateTime>,
    ticks: TimeTicks,
}

impl Touch {
    fn event_msg(&self, dir: bool, colors: ColorTable) -> Msg {
        let (dir_str, color) = match dir {
            true => ("[↑]", colors.up),
            false => ("[↓]", colors.down),
        };

        let normal = format!(
            "{}/{}{}:{}",
            self.args.val_kind.as_str_short(),
            self.args.calc_kind.as_str_short(),
            self.args.ma,
            dir_str,
        );

        let tty = format!(
            "{}/{}{}:{}",
            self.args.val_kind.as_str_short(),
            self.args.calc_kind.as_str_short(),
            self.args.ma,
            dir_str.with(color),
        );

        Msg { normal, tty }
    }
}

impl Touch {
    fn touched(&self, kctx: &KCtx, val: Decimal) -> Option<bool> {
        match self.args.val_kind {
            ExtractKind::PriceClose => self.close_touched(kctx, val),

            ExtractKind::Qty => self.qty_touched(kctx, val),

            ExtractKind::PriceFull | ExtractKind::PriceBody => None,
        }
    }

    fn close_touched(&self, kctx: &KCtx, val: Decimal) -> Option<bool> {
        if kctx.info.raw.price_high >= val && kctx.info.raw.price_low <= val {
            // 开盘价在下，则向上触碰
            Some(kctx.info.raw.price_open < val)
        } else {
            None
        }
    }

    fn qty_touched(&self, kctx: &KCtx, val: Decimal) -> Option<bool> {
        if kctx.info.raw.quantity >= val {
            Some(true)
        } else {
            None
        }
    }

    fn calc(&self, kctx: &KCtx) -> Option<Msg> {
        let ma = kctx.get_val::<Decimal>(&self.ma_key).copied()?;
        let touch_dir = self.touched(kctx, ma)?;
        if self.prev_touched == Some(touch_dir) {
            return None;
        }

        Some(self.event_msg(touch_dir, kctx.colors))
    }

    fn update(&mut self, kctx: &KCtx) -> Option<Msg> {
        let prev_touched = self.prev_touched.take();
        let ma = kctx.get_val::<Decimal>(&self.ma_key).copied()?;

        let touch_dir = self.touched(kctx, ma)?;
        self.prev_touched.replace(touch_dir);
        if prev_touched == Some(touch_dir) {
            return None;
        }

        Some(self.event_msg(touch_dir, kctx.colors))
    }
}

impl Monitor for Touch {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        if let Some(c) = self.checker.as_ref() {
            vec![&self.ma_key, &c.key]
        } else {
            vec![&self.ma_key]
        }
    }

    fn apply(&mut self, kctx: &KCtx) {
        let t = kctx.info.raw.time_begin;
        self.ticks.add(t);

        if kctx.info.raw.finalized {
            self.state.temp.take();
            self.state.perm = self.update(kctx).map(|msg| (t, msg));
        } else {
            let msg_opt = self.calc(kctx);
            // 有新状态，力度确认，且上一个告警没有发生在近期
            if let Some(msg) = msg_opt.as_ref()
                && self.checker.as_ref().map(|c| c.check(kctx)).unwrap_or(true)
                && !self
                    .prev_alert_time
                    .map(|pat| self.ticks.is_recent(pat))
                    .unwrap_or(false)
            {
                self.prev_alert_time.replace(t);
                self.alerts.add(t, msg.clone());
            }

            self.state.temp = msg_opt.map(|m| (t, m));
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
