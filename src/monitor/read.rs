use std::{borrow::Cow, str::FromStr};

use scanf::sscanf;

use crate::{
    impl_builder,
    indicator::base::{BaseExtractorArgs, CalcKind, ExtractKind},
    prelude::*,
};

#[derive(Debug, Clone)]
pub struct ReadArgs {
    val_kind: ExtractKind,
    calc_kind: CalcKind,
    periods: Vec<usize>,
}

impl FromStr for ReadArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut val_kind_str = String::new();
        let mut calc_kind_str = String::new();
        let mut periods_str = String::new();

        sscanf!(s, "read:{val_kind_str},{calc_kind_str},{periods_str}")
            .with_context(|_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse read args"),
            })?;

        let periods = periods_str
            .split(",")
            .map(|s| {
                s.parse::<usize>().map_err(|e| Error::Msg {
                    reason: format!("parse read period: {e}").into(),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let val_kind = val_kind_str.parse()?;
        let calc_kind = calc_kind_str.parse()?;

        Ok(ReadArgs {
            val_kind,
            calc_kind,
            periods,
        })
    }
}

impl Args for ReadArgs {
    type Type = (ExtractKind, CalcKind, Vec<usize>);
    type Target = Read;

    fn new(args: Self::Type) -> Self {
        Self {
            val_kind: args.0,
            calc_kind: args.1,
            periods: args.2,
        }
    }

    fn key(&self) -> String {
        let periods = self
            .periods
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        format!(
            "read:{},{},{}",
            self.val_kind.as_str(),
            self.calc_kind.as_str(),
            periods.join(",")
        )
    }

    fn build(self) -> Result<Self::Target> {
        let ma_keys = self
            .periods
            .iter()
            .copied()
            .map(|p| {
                BaseExtractorArgs::new((self.val_kind, self.calc_kind, p)).key()
            })
            .collect::<Vec<_>>();

        let key = self.key();

        Ok(Read {
            args: self,
            key,
            ma_keys,
            state: Default::default(),
        })
    }
}

impl_builder!(ReadBuilder: ReadArgs => Read);

// cross & stay
pub struct Read {
    args: ReadArgs,
    key: String,
    ma_keys: Vec<String>,
    state: State,
}

impl Read {
    fn read_msg(&self, kctx: &KCtx) -> String {
        let vals = self
            .ma_keys
            .iter()
            .zip(self.args.periods.iter())
            .map(|(key, p)| match kctx.get_val::<Decimal>(key) {
                Some(v) => {
                    format!("{p}:{}", v.round_dp(2))
                }

                None => {
                    format!("{p}:n/a")
                }
            })
            .collect::<Vec<_>>();

        format!(
            "read:{},{}:{}",
            self.args.val_kind.as_str(),
            self.args.calc_kind.as_str(),
            vals.join("|")
        )
    }
}

impl Monitor for Read {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        self.ma_keys.iter().map(|s| s.as_str()).collect()
    }

    fn apply(&mut self, kctx: &KCtx) {
        if kctx.info.raw.finalized {
            self.state.temp.take();
            self.state.perm.replace(self.read_msg(kctx));
        } else {
            self.state.temp.replace(self.read_msg(kctx));
        }
    }

    fn state(&self) -> &State {
        &self.state
    }

    fn terminated(&self) -> bool {
        false
    }
}
