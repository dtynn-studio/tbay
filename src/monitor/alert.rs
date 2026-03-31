use std::collections::{BTreeMap, BTreeSet};

use time::OffsetDateTime;

pub struct AlertManager {
    cap: usize,
    alerts: BTreeMap<OffsetDateTime, String>,
    taken: BTreeSet<OffsetDateTime>,
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new(5)
    }
}

impl AlertManager {
    pub fn new(cap: usize) -> Self {
        AlertManager {
            cap,
            alerts: Default::default(),
            taken: Default::default(),
        }
    }

    pub fn add(&mut self, t: OffsetDateTime, msg: String) {
        if self.alerts.try_insert(t, msg).is_err() {
            return;
        }

        let count = self.alerts.len();
        if count > self.cap {
            for _ in 0..(count - self.cap) {
                if let Some((k, _v)) = self.alerts.pop_first() {
                    self.taken.remove(&k);
                }
            }
        }
    }

    pub fn take(&mut self) -> Vec<(OffsetDateTime, String)> {
        let mut msgs = Vec::new();
        for (k, v) in self.alerts.iter() {
            if self.taken.insert(*k) {
                msgs.push((*k, v.clone()));
            }
        }

        msgs
    }
}
