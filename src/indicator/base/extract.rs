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

fn close_extractor(info: &KInfo) -> Decimal {
    info.raw.price_close
}

fn qty_extractor(info: &KInfo) -> Decimal {
    info.raw.quantity
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
    Qty,
}

impl ExtractKind {
    pub const PRICE_CLOSE_STR: &str = "close";
    pub const QTY_STR: &str = "qty";

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PriceClose => Self::PRICE_CLOSE_STR,
            Self::Qty => Self::QTY_STR,
        }
    }

    pub const fn as_str_short(&self) -> &'static str {
        match self {
            Self::PriceClose => "C",
            Self::Qty => "Q",
        }
    }
}

impl FromStr for ExtractKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::PRICE_CLOSE_STR => Ok(Self::PriceClose),
            Self::QTY_STR => Ok(Self::Qty),
            other => Err(other.unexpected("parse extract kind")),
        }
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
            Self::Sma => "SM",
            Self::Ema => "EM",
            Self::StdDev => "SD",
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
        let extractor = match self.extract {
            ExtractKind::PriceClose => close_extractor,
            ExtractKind::Qty => qty_extractor,
        };

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
            extractor,
            calculator,
        })
    }
}

impl_builder!(BaseExtractorBuilder: BaseExtractorArgs => BaseExtractor);
