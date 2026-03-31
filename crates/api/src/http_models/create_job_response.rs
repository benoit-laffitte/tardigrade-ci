use serde::{Deserialize, Serialize};
use tardigrade_core::JobDefinition;

/// Response payload containing the created job.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateJobResponse {
    pub job: JobDefinition,
}
