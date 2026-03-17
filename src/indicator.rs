use std::any::Any;

use rust_decimal::Decimal;

use crate::prelude::{Builder, KCtx, Result};

pub mod base;
pub mod bollinger;
pub mod cross;
pub mod distance;
pub mod ma;
pub mod macd;
pub mod position;
pub mod stddev;

pub trait Calculator {
    fn calc(&self, next: Decimal) -> Option<Decimal>;
    fn update(&mut self, next: Decimal) -> Option<Decimal>;
}

pub trait Indicator {
    type Output;

    fn key(&self) -> &str;
    fn deps(&self) -> Vec<&str>;
    fn calc(&self, next: &KCtx) -> Option<Self::Output>;
    fn update(&mut self, next: &KCtx) -> Option<Self::Output>;
}

pub trait IndicatorExt: Indicator + Sized + 'static {
    fn wrap_as_any(self) -> Box<dyn Indicator<Output = Box<dyn Any>>> {
        let inner = IndicatorAny(self);
        Box::new(inner)
    }
}

impl<I: Indicator + Sized + 'static> IndicatorExt for I {}

pub struct IndicatorAny<I: Indicator>(I);

impl<I: Indicator> Indicator for IndicatorAny<I>
where
    I::Output: 'static,
{
    type Output = Box<dyn Any>;

    fn key(&self) -> &str {
        self.0.key()
    }

    fn deps(&self) -> Vec<&str> {
        self.0.deps()
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let raw = self.0.calc(next)?;
        Some(Box::new(raw))
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        let raw = self.0.update(next)?;
        Some(Box::new(raw))
    }
}

pub struct BuilderAny<B: Builder> {
    inner: B,
}

impl<B: Builder> BuilderAny<B> {
    pub fn wrap(inner: B) -> Self {
        Self { inner }
    }
}

impl<B: Builder<Target: Indicator> + 'static> Builder for BuilderAny<B> {
    type Target = Box<dyn Indicator<Output = Box<dyn Any>>>;

    fn build(&self, s: &str) -> Result<Self::Target> {
        let raw = self.inner.build(s)?;
        Ok(raw.wrap_as_any())
    }
}
