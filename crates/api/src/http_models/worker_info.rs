use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Worker telemetry model shown by dashboard.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub id: String,
    pub active_builds: usize,
    pub status: String,
    pub last_seen_at: DateTime<Utc>,
}
