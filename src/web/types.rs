use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum StatesKind {
    States,
    Reads,
    Monitors,
}
