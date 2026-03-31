use serde::{Deserialize, Serialize};
use tardigrade_core::JobDefinition;

/// Response payload listing known jobs.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListJobsResponse {
    pub jobs: Vec<JobDefinition>,
}
