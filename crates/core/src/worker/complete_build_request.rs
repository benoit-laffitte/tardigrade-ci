use serde::{Deserialize, Serialize};

use super::WorkerBuildStatus;

/// Contract payload used by workers to report one build completion result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteBuildRequest {
    /// Final worker execution status for the completed build.
    pub status: WorkerBuildStatus,
    /// Optional log line appended when completion is reported.
    pub log_line: Option<String>,
}
