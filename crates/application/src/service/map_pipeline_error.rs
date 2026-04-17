use tardigrade_core::PipelineDslError;

use crate::ApiError;

/// Converts DSL parser/validator failures into API-level invalid pipeline errors.
pub fn map_pipeline_error(error: PipelineDslError) -> ApiError {
    match error {
        PipelineDslError::Yaml(message) => ApiError::InvalidPipeline {
            message: format!("invalid pipeline YAML: {message}"),
            details: None,
        },
        PipelineDslError::Validation(issues) => {
            let summary = issues
                .iter()
                .take(3)
                .map(|issue| format!("{}: {}", issue.field, issue.message))
                .collect::<Vec<_>>()
                .join("; ");
            let suffix = if issues.len() > 3 {
                " (additional issues omitted)"
            } else {
                ""
            };
            ApiError::InvalidPipeline {
                message: format!("pipeline validation failed: {summary}{suffix}"),
                details: Some(issues),
            }
        }
    }
}
