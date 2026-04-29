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
    pub rs: Decimal,
    pub avg_gain: Decimal,
    pub avg_loss: Decimal,
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

impl Indicator for Rsi {
    type Output = RsiValue;

    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![]
    }

    fn calc(&self, _next: &KCtx) -> Option<Self::Output> {
        // RSI can only be meaningfully calculated on finalized K-lines
        // because the price change between consecutive K-lines is not
        // known until both are finalized
        self.current
    }

    fn update(&mut self, next: &KCtx) -> Option<Self::Output> {
        // Get current close and compute change from previous close
        let price_close = next.info.raw.price_close;

        let change = match self.prev_close {
            Some(prev) => price_close - prev,
            None => {
                // First K-line - no previous close to compare
                self.prev_close = Some(price_close);
                return None;
            }
        };
        self.prev_close = Some(price_close);

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

        // Compute or update smoothed averages using Wilder's method
        let (new_avg_gain, new_avg_loss) = (
            (prev_gain * self.smooth_mul + gain) / self.period,
            (prev_loss * self.smooth_mul + loss) / self.period,
        );

        self.prev_avg.replace((new_avg_gain, new_avg_loss));

        let (rsi, rs) = Self::calculate_rsi(new_avg_gain, new_avg_loss);

        let value = RsiValue {
            rsi,
            rs,
            avg_gain: new_avg_gain,
            avg_loss: new_avg_loss,
        };

        self.current.replace(value);
        Some(value)
    }
}

impl Rsi {
    fn calculate_rsi(
        avg_gain: Decimal,
        avg_loss: Decimal,
    ) -> (Decimal, Decimal) {
        let rs = if avg_loss.is_zero() {
            // When avg_loss is zero, RSI = 100 (all gains, no losses)
            Decimal::MAX
        } else {
            avg_gain / avg_loss
        };

        let rsi = if avg_loss.is_zero() {
            Decimal::from(100)
        } else {
            Decimal::from(100) - (Decimal::from(100) / (Decimal::ONE + rs))
        };

        (rsi, rs)
    }
}
