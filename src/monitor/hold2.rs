use std::{borrow::Cow, str::FromStr};

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    config::ColorTable,
    impl_builder,
    indicator::{
        base::{CalcKind, ExtractKind},
        position2::{Pos, Position2, Position2Args},
    },
    monitor::{Msg, alert::AlertManager},
    prelude::*,
};

#[derive(Debug, Clone, Copy)]
pub struct Hold2Args {
    pub calc_kind: CalcKind,
    pub ma: usize,
    pub hold: usize,
    pub for_state: bool,
}

impl FromStr for Hold2Args {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut calc_kind_str = String::new();
        let mut ma = 0usize;
        let mut hold = 0usize;
        let mut for_state = false;

        if sscanf!(s, "hold2:{calc_kind_str},{ma},{hold}").is_err() {
            calc_kind_str.clear();
            ma = 0;
            hold = 0;

            sscanf!(s, "hold2:{calc_kind_str},{ma},{hold},state")
                .with_context(|_| ParseCtx {
                    raw: s.to_owned(),
                    usage: Cow::from("parse hold2 args"),
                })?;

            for_state = true;
        }

        let calc_kind = calc_kind_str.parse()?;

        if ma == 0 {
            return Err(ma.unexpected("ma period"));
        }

        if hold == 0 {
            return Err(ma.unexpected("hold period"));
        }

        Ok(Self {
            calc_kind,
            ma,
            hold,
            for_state,
        })
    }
}

impl Args for Hold2Args {
    type Type = (CalcKind, usize, usize, bool);
    type Target = Hold2;

    fn new(args: Self::Type) -> Self {
        Self {
            calc_kind: args.0,
            ma: args.1,
            hold: args.2,
            for_state: args.3,
        }
    }

    fn key(&self) -> String {
        let state_flag = if self.for_state { ",state" } else { "" };

        format!(
            "hold2:{},{},{}{state_flag}",
            self.calc_kind.as_str(),
            self.ma,
            self.hold
        )
    }

    fn build(self) -> Result<Self::Target> {
        let pos_args = Position2Args::new((self.calc_kind, self.ma));

        let pos_key = pos_args.key();

        let key = self.key();

        Ok(Hold2 {
            args: self,
            key,
            pos_key,
            state: Default::default(),
            alerts: Default::default(),
        })
    }
}

impl_builder!(Hold2Builder: Hold2Args => Hold2);

pub struct Hold2 {
    args: Hold2Args,
    key: String,
    pos_key: String,
    state: State,
    alerts: AlertManager,
}

impl Hold2 {
    fn event_msg(&self, pos: Pos, colors: ColorTable) -> Msg {
        let (pos_str, color) = match pos {
            Pos::Above => ("[▲]", colors.up),
            Pos::Below => ("[▼]", colors.down),
            Pos::Chaos => ("[~]", colors.normal),
        };

        let val_flag = ExtractKind::PriceClose.as_str_short();

        let normal = format!(
            "{val_flag}/{}{}:{pos_str}{}",
            self.args.calc_kind.as_str_short(),
            self.args.ma,
            self.args.hold
        );

        let tty = format!(
            "{val_flag}/{}{}:{}{}",
            self.args.calc_kind.as_str_short(),
            self.args.ma,
            pos_str.with(color),
            self.args.hold
        );

        Msg { normal, tty }
    }

    fn calc(&self, kctx: &KCtx) -> Option<Msg> {
        let val =
            kctx.get_val::<<Position2 as Indicator>::Output>(&self.pos_key)?;
        if val.pos == Pos::Chaos || val.periods <= self.args.hold {
            return None;
        }

        Some(self.event_msg(val.pos, kctx.colors))
    }

    fn update(&mut self, kctx: &KCtx) -> Option<(Msg, bool)> {
        let val =
            kctx.get_val::<<Position2 as Indicator>::Output>(&self.pos_key)?;
        if val.pos == Pos::Chaos || val.periods <= self.args.hold {
            return None;
        }

        Some((
            self.event_msg(val.pos, kctx.colors),
            val.periods == self.args.hold,
        ))
    }
}

impl Monitor for Hold2 {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.pos_key]
    }

    fn apply(&mut self, kctx: &KCtx) {
        let t = kctx.info.t();
        if kctx.info.raw.finalized {
            self.state.clear();
            if let Some((msg, should_alert)) = self.update(kctx) {
                if should_alert {
                    self.alerts.add(t, msg.clone());
                }

                if self.args.for_state {
                    self.state.perm.replace((t, msg));
                }
            }
        } else {
            if self.args.for_state {
                let msg_opt = self.calc(kctx).map(|m| (t, m));
                self.state.temp = msg_opt;
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

    fn is_once(&self) -> bool {
        false
    }
}
