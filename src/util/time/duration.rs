//! copied from humantime

use std::{fmt, time::Duration};

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TBDuration(Duration);

impl fmt::Debug for TBDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl From<Duration> for TBDuration {
    fn from(value: Duration) -> Self {
        Self(value)
    }
}

impl From<humantime::Duration> for TBDuration {
    fn from(value: humantime::Duration) -> Self {
        Self(value.into())
    }
}

impl From<TBDuration> for Duration {
    fn from(value: TBDuration) -> Self {
        value.0
    }
}

impl From<TBDuration> for humantime::Duration {
    fn from(value: TBDuration) -> Self {
        value.0.into()
    }
}

fn item_plural(
    f: &mut fmt::Formatter,
    started: &mut bool,
    name: &str,
    value: u64,
) -> fmt::Result {
    if value > 0 {
        if *started {
            f.write_str(" ")?;
        }
        write!(f, "{}{}", value, name)?;
        if value > 1 {
            f.write_str("s")?;
        }
        *started = true;
    }
    Ok(())
}
fn item(
    f: &mut fmt::Formatter,
    started: &mut bool,
    name: &str,
    value: u32,
) -> fmt::Result {
    if value > 0 {
        if *started {
            f.write_str(" ")?;
        }
        write!(f, "{}{}", value, name)?;
        *started = true;
    }
    Ok(())
}

impl fmt::Display for TBDuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let secs = self.0.as_secs();
        let nanos = self.0.subsec_nanos();

        if secs == 0 && nanos == 0 {
            f.write_str("0s")?;
            return Ok(());
        }

        let years = secs / 31_557_600; // 365.25d
        let ydays = secs % 31_557_600;
        let months = ydays / 2_630_016; // 30.44d
        let mdays = ydays % 2_630_016;
        let days = mdays / 86400;
        let day_secs = mdays % 86400;
        let hours = day_secs / 3600;
        let minutes = day_secs % 3600 / 60;
        let seconds = day_secs % 60;

        let millis = nanos / 1_000_000;
        let micros = nanos / 1000 % 1000;
        let nanosec = nanos % 1000;

        let started = &mut false;
        item_plural(f, started, "y", years)?;
        item_plural(f, started, "M", months)?;
        item_plural(f, started, "d", days)?;
        item(f, started, "h", hours as u32)?;
        item(f, started, "m", minutes as u32)?;
        item(f, started, "s", seconds as u32)?;
        item(f, started, "ms", millis)?;
        item(f, started, "us", micros)?;
        item(f, started, "ns", nanosec)?;
        Ok(())
    }
}
