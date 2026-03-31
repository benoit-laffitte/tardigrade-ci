use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::PipelineRetryPolicy;

/// Single execution unit in a stage with optional retry override.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineStep {
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub retry: Option<PipelineRetryPolicy>,
}

impl PipelineStep {
    /// Creates one pipeline step with image and command.
    pub fn new(name: impl Into<String>, image: impl Into<String>, command: Vec<String>) -> Self {
        Self {
            name: name.into(),
            image: image.into(),
            command,
            env: BTreeMap::new(),
            retry: None,
        }
    }
}
