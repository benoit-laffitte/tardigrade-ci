use serde::{Deserialize, Serialize};
use tardigrade_core::BuildRecord;

/// Response payload containing enqueued build record.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunJobResponse {
    pub build: BuildRecord,
}
