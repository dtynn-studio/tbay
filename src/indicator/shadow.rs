use std::borrow::Cow;

use scanf::sscanf;

use crate::{impl_builder, prelude::*};

#[derive(Clone, Copy)]
pub struct ShadowValue {
    pub ratio: Decimal,
    pub abs: Decimal,
    pub is_above: bool,
}

#[derive(Clone, Copy)]
pub struct ShadowArgs {
    pub threshold: f64,
}

impl FromStr for ShadowArgs {
    type Err = Error;

    // format: "shadow:{threshold}"
    // examples: "shadow:0.6"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut threshold = 0.0f64;

        sscanf!(s, "shadow:{threshold}").with_context(|_| ParseCtx {
            raw: s.to_owned(),
            usage: Cow::from("parse shadow args"),
        })?;

        Ok(Self { threshold })
    }
}

impl Args for ShadowArgs {
    type Type = f64;
    type Target = Shadow;

    fn new(args: Self::Type) -> Self {
        Self { threshold: args }
    }

    fn key(&self) -> String {
        format!("shadow:{}", self.threshold)
    }

    fn build(self) -> Result<Self::Target> {
        let threshold =
            Decimal::from_f64(self.threshold).required("shadow threshold")?;
        let key = self.key();

        Ok(Shadow {
            key,
            _args: self,
            threshold,
        })
    }
}

impl_builder!(ShadowBuilder: ShadowArgs => Shadow);

pub struct Shadow {
    key: String,
    _args: ShadowArgs,
    threshold: Decimal,
}

impl Indicator for Shadow {
    type Output = ShadowValue;

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![]
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        if next.info.full.height.is_zero() {
            return None;
        }

        let mut val = None;

        if next.info.shadow.above >= next.info.shadow.below {
            let above_ratio = next.info.shadow.above / next.info.full.height;
            if above_ratio >= self.threshold {
                val.replace(ShadowValue {
                    ratio: above_ratio,
                    abs: next.info.shadow.above,
                    is_above: true,
                });
            }
        } else {
            let below_ratio = next.info.shadow.below / next.info.full.height;
            if below_ratio >= self.threshold {
                val.replace(ShadowValue {
                    ratio: below_ratio,
                    abs: next.info.shadow.below,
                    is_above: false,
                });
            }
        }

        val
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        self.calc(next)
    }
}
