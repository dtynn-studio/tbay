use std::borrow::Cow;

use crate::{
    impl_builder,
    indicator::{
        Builder, Calculator, Indicator,
        base::{BaseExtractorArgs, CalcKind, ExtractKind},
        cross::{Cross, CrossItem, CrossValue},
        ma::Ema,
    },
    prelude::{
        Args, Decimal, Error, FromStr, KCtx, ParseCtx, Result, ResultExt,
        Unexpected,
    },
};

#[derive(Clone)]
pub struct MacdValue {
    pub dif: Decimal,
    pub dea: Decimal,
    pub macd: Decimal,
    pub cross: Option<CrossValue<Decimal>>,
}

pub struct Macd {
    key: String,
    fast_ma_key: String,
    slow_ma_key: String,
    dea: Ema,
    current: Option<MacdValue>,
    cross: Cross<Decimal>,
}

#[derive(Clone, Copy, Debug)]
pub struct MacdArgs {
    pub fast: usize,
    pub slow: usize,
    pub dea_period: usize,
}

impl FromStr for MacdArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut fast = 0usize;
        let mut slow = 0usize;
        let mut dea_period = 0usize;

        scanf::sscanf!(s, "macd:{fast},{slow},{dea_period}").with_context(
            |_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse Macd"),
            },
        )?;

        if fast == 0 {
            return Err(fast.unexpected("macd fast period"));
        }

        if slow == 0 {
            return Err(slow.unexpected("macd slow period"));
        }

        if dea_period == 0 {
            return Err(dea_period.unexpected("macd dea period"));
        }

        Ok(Self {
            fast,
            slow,
            dea_period,
        })
    }
}

impl Args for MacdArgs {
    type Type = (usize, usize, usize);
    type Target = Macd;

    fn new(args: Self::Type) -> Self {
        Self {
            fast: args.0,
            slow: args.1,
            dea_period: args.2,
        }
    }

    fn key(&self) -> String {
        format!("macd:{},{},{}", self.fast, self.slow, self.dea_period)
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();

        let fast_ma_key = BaseExtractorArgs::new((
            ExtractKind::PriceClose,
            CalcKind::Ema,
            self.fast,
        ))
        .key();

        let slow_ma_key = BaseExtractorArgs::new((
            ExtractKind::PriceClose,
            CalcKind::Ema,
            self.slow,
        ))
        .key();

        let dea = Ema::new(self.dea_period);

        Ok(Macd {
            key,
            fast_ma_key,
            slow_ma_key,
            dea,
            current: None,
            cross: Cross::default(),
        })
    }
}

impl_builder!(MacdBuilder: MacdArgs => Macd);

impl Indicator for Macd {
    type Output = MacdValue;

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![&self.fast_ma_key, &self.slow_ma_key]
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let fast = *next.get_val::<Decimal>(&self.fast_ma_key)?;
        let slow = *next.get_val::<Decimal>(&self.slow_ma_key)?;
        let dif = fast - slow;
        let dea = self.dea.calc(dif)?;
        let cross = self.cross.calc(CrossItem::new(dif, dea));

        Some(MacdValue {
            dif,
            dea,
            macd: dif - dea,
            cross,
        })
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        let fast = *next.get_val::<Decimal>(&self.fast_ma_key)?;
        let slow = *next.get_val::<Decimal>(&self.slow_ma_key)?;
        let dif = fast - slow;
        let dea = self.dea.update(dif)?;

        let cross = self.cross.update(CrossItem::new(dif, dea));

        let value = MacdValue {
            dif,
            dea,
            macd: dif - dea,
            cross,
        };

        self.current.replace(value.clone());
        Some(value)
    }
}
