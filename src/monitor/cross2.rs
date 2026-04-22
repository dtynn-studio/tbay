use std::borrow::Cow;

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    impl_builder,
    indicator::{
        base::{BaseExtractorArgs, CalcKind, ExtractKind},
        cross::{CrossValue, MaCrossArgs},
    },
    monitor::alert::AlertManager,
    prelude::*,
};

// cross2:close,ema,5,20,60
#[derive(Clone, Copy)]
pub struct Cross2Args {
    val_kind: ExtractKind,
    calc_kind: CalcKind,
    fast: usize,
    slow: usize,
    base: usize,
}

impl FromStr for Cross2Args {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut val_kind_str = String::new();
        let mut calc_kind_str = String::new();
        let mut fast = 0usize;
        let mut slow = 0usize;
        let mut base = 0usize;

        if sscanf!(s, "cross2:{val_kind_str},{calc_kind_str},{fast},{slow}")
            .is_err()
        {
            sscanf!(
                s,
                "cross2:{val_kind_str},{calc_kind_str},{fast},{slow},{base}"
            )
            .with_context(|_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse cross2 args"),
            })?;
        }

        let val_kind = val_kind_str.parse()?;
        let calc_kind = calc_kind_str.parse()?;

        Ok(Self {
            val_kind,
            calc_kind,
            fast,
            slow,
            base,
        })
    }
}

impl Args for Cross2Args {
    type Type = (ExtractKind, CalcKind, usize, usize, usize);
    type Target = Cross2;

    fn new(args: Self::Type) -> Self {
        Self {
            val_kind: args.0,
            calc_kind: args.1,
            fast: args.2,
            slow: args.3,
            base: args.4,
        }
    }

    fn key(&self) -> String {
        format!(
            "cross2:{},{},{},{},{}",
            self.val_kind.as_str(),
            self.calc_kind.as_str(),
            self.fast,
            self.slow,
            self.base,
        )
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();
        let cross_key = MaCrossArgs::new((
            ExtractKind::PriceClose,
            self.calc_kind,
            self.fast,
            self.slow,
        ))
        .key();
        let base_ma_key = BaseExtractorArgs::new((
            ExtractKind::PriceClose,
            self.calc_kind,
            self.base,
        ))
        .key();

        Ok(Cross2 {
            args: self,
            key,
            cross_key,
            base_ma_key,
            state: Default::default(),
            alert: Default::default(),
        })
    }
}

impl_builder!(Cross2Builer: Cross2Args => Cross2);

pub struct Cross2 {
    args: Cross2Args,
    key: String,
    cross_key: String,
    base_ma_key: String,
    state: State,
    alert: AlertManager,
}

impl Cross2 {
    fn calc(&self, kctx: &KCtx) -> Option<Msg> {
        let cross_val = kctx.get_val::<CrossValue<Decimal>>(&self.cross_key)?;
        let direction = cross_val.cross?;
        let ma_val = kctx.get_val::<Decimal>(&self.base_ma_key).copied()?;

        let (dir_flag, dir_color) = if direction {
            (format!("[↗]{}", self.args.slow), kctx.colors.up)
        } else {
            (format!("[↘]{}", self.args.slow), kctx.colors.down)
        };

        let cross_point = cross_val.next.slow;
        let is_above = cross_point >= ma_val;
        let (relative_pos_flag, relative_pos_color) = if is_above {
            ("[▲]", kctx.colors.up)
        } else {
            ("[▼]", kctx.colors.down)
        };

        let normal = format!(
            "{}/{}{}:{dir_flag}{}{relative_pos_flag}{}",
            self.args.val_kind.as_str_short(),
            self.args.calc_kind.as_str_short(),
            self.args.fast,
            self.args.slow,
            self.args.base,
        );

        let tty = format!(
            "{}/{}{}:{}{}{}{}",
            self.args.val_kind.as_str_short(),
            self.args.calc_kind.as_str_short(),
            self.args.fast,
            dir_flag.with(dir_color),
            self.args.slow,
            relative_pos_flag.with(relative_pos_color),
            self.args.base,
        );

        Some(Msg { normal, tty })
    }
}

impl Monitor for Cross2 {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.cross_key, &self.base_ma_key]
    }

    fn apply(&mut self, kctx: &KCtx) {
        let t = kctx.info.t();
        let msg_opt = self.calc(kctx).map(|m| (t, m));
        if kctx.info.raw.finalized {
            self.state.temp.take();
            self.state.perm = msg_opt;
            if let Some((t, m)) = self.state.perm.as_ref() {
                self.alert.add(*t, m.clone());
            }
        } else {
            self.state.temp = msg_opt;
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
