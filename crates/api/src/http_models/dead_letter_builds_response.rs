use serde::{Deserialize, Serialize};
use tardigrade_core::BuildRecord;

/// Response payload listing builds moved to dead-letter set.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeadLetterBuildsResponse {
    pub builds: Vec<BuildRecord>,
}
