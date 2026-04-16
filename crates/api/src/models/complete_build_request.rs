use serde::{Deserialize, Serialize};

use super::WorkerBuildStatus;

/// Request payload for worker completion call.
#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteBuildRequest {
    pub status: WorkerBuildStatus,
    pub log_line: Option<String>,
}
