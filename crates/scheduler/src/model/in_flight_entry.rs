use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// In-flight lease metadata for ownership and stale-claim detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct InFlightEntry {
    pub(crate) worker_id: String,
    pub(crate) claimed_at: DateTime<Utc>,
}
