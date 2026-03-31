use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

use super::InFlightEntry;

/// Serializable queue state used by in-memory and file-backed schedulers.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct QueueState {
    pub(crate) queue: VecDeque<Uuid>,
    pub(crate) in_flight: HashMap<Uuid, InFlightEntry>,
}
