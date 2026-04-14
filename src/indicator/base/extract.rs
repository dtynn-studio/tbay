use std::{borrow::Cow, str::FromStr};

use scanf::sscanf;

use crate::{
    impl_builder,
    indicator::{
        Calculator,
        base::BaseCalculator,
        ma::{Ema, Sma},
        stddev::StdDev,
    },
    prelude::{
        Args, Builder, Decimal, Error, Indicator, KCtx, KInfo, ParseCtx,
        Result, ResultExt, Unexpected,
    },
};

pub mod extractor {
    use crate::prelude::{Decimal, KInfo};

    pub fn close(info: &KInfo) -> Decimal {
        info.raw.price_close
    }

    pub fn qty(info: &KInfo) -> Decimal {
        info.raw.quantity
    }

    pub fn full(info: &KInfo) -> Decimal {
        info.full.height
    }

    pub fn body(info: &KInfo) -> Decimal {
        info.body.height
    }
}

pub struct BaseExtractor {
    key: String,
    calculator: BaseCalculator,
    extractor: fn(&KInfo) -> Decimal,
}

impl Indicator for BaseExtractor {
    type Output = Decimal;

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![]
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let value = (self.extractor)(&next.info);
        self.calculator.calc(value)
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        let value = (self.extractor)(&next.info);
        self.calculator.update(value)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ExtractKind {
    PriceClose,
    PriceFull,
    PriceBody,
    Qty,
}

impl ExtractKind {
    pub const PRICE_CLOSE_STR: &str = "close";
    pub const PRICE_FULL_STR: &str = "full";
    pub const PRICE_BODY_STR: &str = "body";
    pub const QTY_STR: &str = "qty";

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PriceClose => Self::PRICE_CLOSE_STR,
            Self::PriceFull => Self::PRICE_FULL_STR,
            Self::PriceBody => Self::PRICE_BODY_STR,
            Self::Qty => Self::QTY_STR,
        }
    }

    pub const fn as_str_short(&self) -> &'static str {
        match self {
            Self::PriceClose => "C",
            Self::PriceFull => "F",
            Self::PriceBody => "B",
            Self::Qty => "Q",
        }
    }

    pub const fn extractor(self) -> fn(&KInfo) -> Decimal {
        match self {
            Self::PriceClose => extractor::close,
            Self::PriceFull => extractor::full,
            Self::PriceBody => extractor::body,
            Self::Qty => extractor::qty,
        }
    }
}

impl FromStr for ExtractKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::PRICE_CLOSE_STR => Ok(Self::PriceClose),
            Self::PRICE_FULL_STR => Ok(Self::PriceFull),
            Self::PRICE_BODY_STR => Ok(Self::PriceBody),
            Self::QTY_STR => Ok(Self::Qty),
            other => Err(other.unexpected("parse extract kind")),
        }
    }
}

impl std::fmt::Display for ExtractKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CalcKind {
    Sma,
    Ema,
    StdDev,
}

impl CalcKind {
    pub const SMA_STR: &str = "sma";
    pub const EMA_STR: &str = "ema";
    pub const STD_DEV_STR: &str = "stddev";

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Sma => Self::SMA_STR,
            Self::Ema => Self::EMA_STR,
            Self::StdDev => Self::STD_DEV_STR,
        }
    }

    pub const fn as_str_short(&self) -> &'static str {
        match self {
            Self::Sma => "S",
            Self::Ema => "E",
            Self::StdDev => "D",
        }
    }
}

impl FromStr for CalcKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::SMA_STR => Ok(Self::Sma),
            Self::EMA_STR => Ok(Self::Ema),
            Self::STD_DEV_STR => Ok(Self::StdDev),
            other => Err(other.unexpected("parse calc kind")),
        }
    }
}

#[derive(Clone)]
pub struct BaseExtractorArgs {
    extract: ExtractKind,
    calc: CalcKind,
    period: usize,
}

impl FromStr for BaseExtractorArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut extract_kind: String = String::new();
        let mut calc_kind: String = String::new();
        let mut period = 0usize;

        sscanf!(s, "{extract_kind}:{calc_kind}:{period}").with_context(
            |_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse PriceMa"),
            },
        )?;

        let extract = extract_kind.parse()?;

        let calc = calc_kind.parse()?;

        if period == 0 {
            return Err(period.unexpected("period"));
        }

        Ok(Self {
            extract,
            calc,
            period,
        })
    }
}

impl Args for BaseExtractorArgs {
    type Type = (ExtractKind, CalcKind, usize);
    type Target = BaseExtractor;

    fn new(args: Self::Type) -> Self {
        Self {
            extract: args.0,
            calc: args.1,
            period: args.2,
        }
    }

    fn key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.extract.as_str(),
            self.calc.as_str(),
            self.period
        )
    }

    fn build(self) -> Result<Self::Target> {
        let calculator = match self.calc {
            CalcKind::Sma => BaseCalculator::Sma(Sma::new(self.period)),
            CalcKind::Ema => BaseCalculator::Ema(Ema::new(self.period)),
            CalcKind::StdDev => {
                BaseCalculator::StdDev(StdDev::new(self.period))
            }
        };

        let key = self.key();

        Ok(BaseExtractor {
            key,
            extractor: self.extract.extractor(),
            calculator,
        })
    }
}

impl_builder!(BaseExtractorBuilder: BaseExtractorArgs => BaseExtractor);
