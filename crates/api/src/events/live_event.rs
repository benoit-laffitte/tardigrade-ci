use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Live event model emitted by the API and streamed to dashboard clients.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveEvent {
    pub kind: String,
    pub severity: String,
    pub message: String,
    pub job_id: Option<Uuid>,
    pub build_id: Option<Uuid>,
    pub worker_id: Option<String>,
    pub at: DateTime<Utc>,
}
