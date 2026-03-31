use serde::{Deserialize, Serialize};

/// One actionable validation issue found in a pipeline definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineValidationIssue {
    pub field: String,
    pub message: String,
}
