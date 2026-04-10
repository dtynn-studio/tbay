use std::borrow::Cow;

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
pub struct TouchArgs {
    val_kind: ExtractKind,
    calc_kind: CalcKind,
    ma: usize,
}

impl FromStr for TouchArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut val_kind_str = String::new();
        let mut calc_kind_str = String::new();
        let mut ma = 0usize;

        sscanf!(s, "touch:{val_kind_str},{calc_kind_str},{ma}").with_context(
            |_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse touch args"),
            },
        )?;

        let val_kind = val_kind_str.parse()?;
        let calc_kind = calc_kind_str.parse()?;

        Ok(Self {
            val_kind,
            calc_kind,
            ma,
        })
    }
}

impl Args for TouchArgs {
    type Type = (ExtractKind, CalcKind, usize);
    type Target = Touch;

    fn new(args: Self::Type) -> Self {
        Self {
            val_kind: args.0,
            calc_kind: args.1,
            ma: args.2,
        }
    }

    fn key(&self) -> String {
        format!(
            "touch:{},{},{}",
            self.val_kind.as_str(),
            self.calc_kind.as_str(),
            self.ma
        )
    }

    fn build(self) -> Result<Self::Target> {
        let ma_args =
            BaseExtractorArgs::new((self.val_kind, self.calc_kind, self.ma));
        let ma_key = ma_args.key();
        let key = self.key();

        Ok(Touch {
            args: self,
            key,
            ma_key,
            prev_touched: None,
            state: Default::default(),
            alerts: Default::default(),
            temp_t: None,
        })
    }
}

impl_builder!(TouchBuilder: TouchArgs => Touch);

pub struct Touch {
    args: TouchArgs,
    key: String,
    ma_key: String,
    prev_touched: Option<bool>,
    state: State,
    alerts: AlertManager,
    temp_t: Option<OffsetDateTime>,
}

impl Touch {
    fn event_msg(&self, dir: bool, colors: ColorTable) -> Msg {
        let (dir_str, color) = match dir {
            true => ("↑", colors.up),
            false => ("↓", colors.down),
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
        vec![&self.ma_key]
    }

    fn apply(&mut self, kctx: &KCtx) {
        if kctx.info.raw.finalized {
            self.state.temp.take();
            self.state.perm =
                self.update(kctx).map(|msg| (kctx.info.raw.time_begin, msg));
        } else {
            let prev_temp_t = self.temp_t.replace(kctx.info.raw.time_begin);
            if prev_temp_t != self.temp_t || self.state.temp.is_none() {
                self.state.temp =
                    self.calc(kctx).map(|msg| (kctx.info.raw.time_begin, msg));

                if let Some((t, msg)) = self.state.temp.as_ref().cloned() {
                    self.alerts.add(t, msg);
                }
            }
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
