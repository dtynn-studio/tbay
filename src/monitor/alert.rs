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

        let period = kctx.info.raw.time_end - t;
        let check_duration = period / 3;

        let gap = kctx.info.raw.time_end - now;
        if gap >= check_duration {
            return false;
        }

        self.prev_t.replace(t);

        true
    }
}
