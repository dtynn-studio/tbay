use crate::{
    indicator::{
        Indicator,
        base::{BaseExtractorBuilder, CalcKind, ExtractKind},
    },
    prelude::{
        Builder, Decimal, Error, FromStr, KCtx, ParseCtx, Result, Unexpected,
    },
};
use snafu::ResultExt;
use std::borrow::Cow;

pub struct Distance {
    key: String,
    // close:ema:5
    key1: String,
    // close:ema:20
    key2: String,
    current: Option<Decimal>,
}

impl Indicator for Distance {
    type Output = Decimal;

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.key1, &self.key2]
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let val1 = *next.get_val::<Decimal>(&self.key1)?;
        let val2 = *next.get_val::<Decimal>(&self.key2)?;

        Some((val1 - val2).abs())
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        let calculated = self.calc(next)?;
        self.current.replace(calculated);
        Some(calculated)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DistanceBuilder {
    kind: CalcKind,
    ma1: usize,
    ma2: usize,
}

impl FromStr for DistanceBuilder {
    type Err = Error;

    // key format: distance:sma,5,20
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut kind_str = String::new();
        let mut ma1 = 0usize;
        let mut ma2 = 0usize;

        scanf::sscanf!(s, "distance:{kind_str},{ma1},{ma2}").with_context(
            |_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse Distance"),
            },
        )?;

        let kind = match kind_str.as_str() {
            "sma" => CalcKind::Sma,
            "ema" => CalcKind::Ema,
            other => return Err(other.unexpected("distance kind")),
        };

        if ma1 == 0 {
            return Err(ma1.unexpected("distance ma1 period"));
        }

        if ma2 == 0 {
            return Err(ma2.unexpected("distance ma2 period"));
        }

        Ok(Self { kind, ma1, ma2 })
    }
}

impl Builder for DistanceBuilder {
    type Args = (CalcKind, usize, usize);
    type Target = Distance;

    fn new(args: Self::Args) -> Self {
        Self {
            kind: args.0,
            ma1: args.1,
            ma2: args.2,
        }
    }

    fn key(&self) -> String {
        format!("distance:{},{},{}", self.kind.as_str(), self.ma1, self.ma2)
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();

        let key1 = BaseExtractorBuilder::new((
            ExtractKind::PriceClose,
            self.kind,
            self.ma1,
        ))
        .key();

        let key2 = BaseExtractorBuilder::new((
            ExtractKind::PriceClose,
            self.kind,
            self.ma2,
        ))
        .key();

        Ok(Distance {
            key,
            key1,
            key2,
            current: None,
        })
    }
}
