use std::{sync::LazyLock, time::Duration};

use time::{OffsetDateTime, UtcOffset};

use crate::prelude::*;

mod duration;

pub use duration::TBDuration;

pub const MILLI_SEC: i64 = 1000;

pub static LOCAL_OFFSET: LazyLock<UtcOffset> = LazyLock::new(|| {
    UtcOffset::current_local_offset().expect("local time offset")
});

pub fn local_from_unix_timestamp_millis_truncated(
    field: &'static str,
    ts: i64,
) -> Result<OffsetDateTime> {
    OffsetDateTime::from_unix_timestamp(ts / 1000)
        .with_context(|_| DatetimeCtx { field })
        .map(|t| t.to_offset(*LOCAL_OFFSET))
}

pub fn truncate(
    field: &'static str,
    t: OffsetDateTime,
    d: Duration,
) -> Result<OffsetDateTime> {
    let dsecs = d.as_secs() as i64;
    if dsecs <= 0 {
        return Ok(t);
    }

    let ts = t.unix_timestamp() / dsecs * dsecs;
    OffsetDateTime::from_unix_timestamp(ts)
        .with_context(|_| DatetimeCtx { field })
        .map(|t| t.to_offset(*LOCAL_OFFSET))
}

pub fn format_hhmm(t: &OffsetDateTime) -> String {
    format!("{:02}:{:02}", t.hour(), t.minute())
}
