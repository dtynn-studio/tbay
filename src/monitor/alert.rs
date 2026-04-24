use std::time::Duration;

use time::OffsetDateTime;

use crate::prelude::*;

#[derive(Default)]
pub struct AlertManager {
    alerts: Option<(OffsetDateTime, Msg)>,
}

impl AlertManager {
    pub fn add(&mut self, t: OffsetDateTime, msg: Msg) {
        self.alerts = Some((t, msg));
    }

    pub fn take(&mut self) -> Vec<(OffsetDateTime, Msg)> {
        if let Some(tm) = self.alerts.take() {
            vec![tm]
        } else {
            vec![]
        }
    }
}

const TEMP_ALERT_GAP: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct TempAlertChecker {
    prev_t: Option<OffsetDateTime>,
}

impl TempAlertChecker {
    pub fn allow(&mut self, kctx: &KCtx) -> bool {
        let t = kctx.info.t();
        if kctx.info.raw.finalized || self.prev_t == Some(t) {
            return false;
        }

        let Ok(now) = OffsetDateTime::now_local() else {
            return false;
        };

        let gap = kctx.info.raw.time_end - now;
        if gap > TEMP_ALERT_GAP {
            return false;
        }

        self.prev_t.replace(t);

        true
    }
}
