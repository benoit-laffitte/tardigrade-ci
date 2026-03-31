use serde::{Deserialize, Serialize};

/// One actionable validation issue found in a technology profile model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TechnologyProfileValidationIssue {
    pub field: String,
    pub message: String,
}
