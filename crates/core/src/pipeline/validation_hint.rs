use serde::{Deserialize, Serialize};

/// One non-blocking recommendation emitted for pipeline quality improvements.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineValidationHint {
    pub field: String,
    pub message: String,
}
