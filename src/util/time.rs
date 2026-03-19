use std::sync::LazyLock;

use time::{OffsetDateTime, UtcOffset};

use crate::prelude::*;

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
