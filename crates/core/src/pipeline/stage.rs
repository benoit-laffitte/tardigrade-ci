use serde::{Deserialize, Serialize};

use super::PipelineStep;

/// Ordered stage grouping one or more executable steps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineStage {
    pub name: String,
    pub steps: Vec<PipelineStep>,
}

impl PipelineStage {
    /// Creates one pipeline stage with ordered steps.
    pub fn new(name: impl Into<String>, steps: Vec<PipelineStep>) -> Self {
        Self {
            name: name.into(),
            steps,
        }
    }
}
