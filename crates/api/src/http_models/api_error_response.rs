use serde::{Deserialize, Serialize};
use tardigrade_core::PipelineValidationIssue;

/// Structured API error response with optional detailed issues.
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub code: String,
    pub message: String,
    pub details: Option<Vec<PipelineValidationIssue>>,
}
