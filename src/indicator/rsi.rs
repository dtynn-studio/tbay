use std::borrow::Cow;

use crate::{
    impl_builder,
    indicator::{Builder, Calculator, Indicator, ma::Sma},
    prelude::{
        Args, Decimal, Error, FromStr, KCtx, ParseCtx, Result, ResultExt,
        Unexpected,
    },
};

#[derive(Clone, Copy)]
pub struct RsiValue {
    pub rsi: Decimal,
    pub avg_gain: Decimal,
    pub avg_loss: Decimal,
}

#[derive(Clone, Copy, Debug)]
pub struct RsiArgs {
    pub period: usize,
}

impl FromStr for RsiArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut period = 0usize;

        scanf::sscanf!(s, "rsi:{period}").with_context(|_| ParseCtx {
            raw: s.to_owned(),
            usage: Cow::from("parse Rsi"),
        })?;

        if period == 0 {
            return Err(period.unexpected("rsi period"));
        }

        Ok(Self { period })
    }
}

impl Args for RsiArgs {
    type Type = usize;
    type Target = Rsi;

    fn new(period: Self::Type) -> Self {
        Self { period }
    }

    fn key(&self) -> String {
        format!("rsi:{}", self.period)
    }

    fn build(self) -> Result<Self::Target> {
        Ok(Rsi {
            _args: self,
            key: self.key(),
            period: Decimal::from(self.period),
            smooth_mul: Decimal::from(self.period) - Decimal::ONE,
            prev_close: None,
            gains: Sma::new(self.period),
            losses: Sma::new(self.period),
            prev_avg: None,
            current: None,
        })
    }
}

impl_builder!(RsiBuilder: RsiArgs => Rsi);

fn calculate_rsi(avg_gain: Decimal, avg_loss: Decimal) -> Decimal {
    if avg_loss.is_zero() {
        Decimal::from(100)
    } else {
        Decimal::from(100)
            - (Decimal::from(100) / (Decimal::ONE + avg_gain / avg_loss))
    }
}

pub struct Rsi {
    _args: RsiArgs,
    key: String,
    period: Decimal,
    smooth_mul: Decimal,
    prev_close: Option<Decimal>,
    gains: Sma,
    losses: Sma,
    prev_avg: Option<(Decimal, Decimal)>,
    current: Option<RsiValue>,
}

impl Rsi {
    fn calculate_rsi(
        &self,
        prev_gain: Decimal,
        gain: Decimal,
        prev_loss: Decimal,
        loss: Decimal,
    ) -> RsiValue {
        let (new_avg_gain, new_avg_loss) = (
            (prev_gain * self.smooth_mul + gain) / self.period,
            (prev_loss * self.smooth_mul + loss) / self.period,
        );

        let rsi = calculate_rsi(new_avg_gain, new_avg_loss);

        RsiValue {
            rsi,
            avg_gain: new_avg_gain,
            avg_loss: new_avg_loss,
        }
    }
}

impl Indicator for Rsi {
    type Output = RsiValue;

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![]
    }

    fn calc(&self, next: &KCtx) -> Option<Self::Output> {
        let prev_close = self.prev_close?;
        let (prev_gain, prev_loss) = self.prev_avg?;

        let price_close = next.info.raw.price_close;
        let change = price_close - prev_close;

        // Separate gain and loss
        let (gain, loss) = if change >= Decimal::ZERO {
            (change, Decimal::ZERO)
        } else {
            (Decimal::ZERO, -change)
        };

        Some(self.calculate_rsi(prev_gain, gain, prev_loss, loss))
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        // Get current close and compute change from previous close
        let price_close = next.info.raw.price_close;
        let prev_close = self.prev_close.replace(price_close)?;

        let change = price_close - prev_close;

        // Separate gain and loss
        let (gain, loss) = if change > Decimal::ZERO {
            (change, Decimal::ZERO)
        } else if change < Decimal::ZERO {
            (Decimal::ZERO, -change)
        } else {
            (Decimal::ZERO, Decimal::ZERO)
        };

        // Update Sma
        let Some((prev_gain, prev_loss)) = self.prev_avg else {
            let gain_updated = self.gains.update(gain);
            let loss_updated = self.losses.update(loss);
            let avg = gain_updated.zip(loss_updated)?;
            self.prev_avg.replace(avg);
            return None;
        };

        let val = self.calculate_rsi(prev_gain, gain, prev_loss, loss);
        self.prev_avg.replace((val.avg_gain, val.avg_loss));
        self.current.replace(val);
        Some(val)
    }
}
