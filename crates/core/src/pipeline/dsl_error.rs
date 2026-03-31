use std::fmt;

use super::PipelineValidationIssue;

/// Error returned when parsing or validating pipeline DSL input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineDslError {
    Yaml(String),
    Validation(Vec<PipelineValidationIssue>),
}

impl fmt::Display for PipelineDslError {
    /// Renders parser/validator failures in operator-friendly text form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yaml(message) => write!(f, "invalid YAML pipeline definition: {message}"),
            Self::Validation(issues) => write!(
                f,
                "invalid pipeline definition ({} structural issue(s))",
                issues.len()
            ),
        }
    }
}

impl std::error::Error for PipelineDslError {}
