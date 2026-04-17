use serde::{Deserialize, Serialize};

/// Worker-reported terminal result for one build execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerBuildStatus {
    /// Build execution completed successfully.
    Success,
    /// Build execution failed.
    Failed,
}
