use time::OffsetDateTime;

use crate::monitor::Msg;

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
