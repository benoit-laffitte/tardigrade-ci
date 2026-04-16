use serde::{Deserialize, Serialize};

/// Worker-reported terminal result for one build execution.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerBuildStatus {
    Success,
    Failed,
}
