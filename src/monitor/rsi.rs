use std::borrow::Cow;

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    impl_builder,
    indicator::rsi::{self, RsiValue},
    monitor::{State, alert::AlertManager},
    prelude::*,
};

#[derive(Clone, Copy)]
pub struct RsiArgs {
    periods: usize,
    weak: usize,
    strong: usize,
}

impl FromStr for RsiArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut periods = 0usize;
        let mut weak = 0usize;
        let mut strong = 0usize;

        sscanf!(s, "rsi:{periods},{weak},{strong}").with_context(|_| {
            ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse rsi args"),
            }
        })?;

        Ok(Self {
            periods,
            weak,
            strong,
        })
    }
}

impl Args for RsiArgs {
    type Type = (usize, usize, usize);
    type Target = Rsi;

    fn new(args: Self::Type) -> Self {
        Self {
            periods: args.0,
            weak: args.1,
            strong: args.2,
        }
    }

    fn key(&self) -> String {
        format!("rsi:{},{},{}", self.periods, self.weak, self.strong)
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();
        let rsi_key = rsi::RsiArgs::new(self.periods).key();

        Ok(Rsi {
            _args: self,
            key,
            rsi_key,
            weak: Decimal::from(self.weak),
            strong: Decimal::from(self.strong),
            state: Default::default(),
            alerts: Default::default(),
        })
    }
}

impl_builder!(RsiBuilder: RsiArgs => Rsi);

pub struct Rsi {
    _args: RsiArgs,
    key: String,
    rsi_key: String,
    weak: Decimal,
    strong: Decimal,
    state: State,
    alerts: AlertManager,
}

impl Rsi {
    fn generate_msg(&self, val: RsiValue, kctx: &KCtx) -> Option<Msg> {
        let (flag, flag_color) = if val.rsi >= self.strong {
            ("[▲]", kctx.colors.up)
        } else if val.rsi <= self.weak {
            ("[▼]", kctx.colors.down)
        } else {
            return None;
        };

        let rsi = val.rsi.round_dp(1);
        let normal = format!("🪜:{flag}{rsi}");
        let tty = format!("🪜:{}{rsi}", flag.with(flag_color));

        Some(Msg { normal, tty })
    }
}

impl Monitor for Rsi {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.rsi_key]
    }

    fn apply(&mut self, kctx: &KCtx) {
        let t = kctx.info.t();
        let val = kctx.get_val::<RsiValue>(&self.rsi_key).copied();
        let msg_opt =
            val.and_then(|v| self.generate_msg(v, kctx).map(|m| (t, m)));

        if kctx.info.raw.finalized {
            if let Some((t, m)) = msg_opt.as_ref().cloned() {
                self.alerts.add(t, m);
            }

            self.state.clear();
            self.state.perm = msg_opt;
        } else {
            self.state.temp = msg_opt;
        };
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
