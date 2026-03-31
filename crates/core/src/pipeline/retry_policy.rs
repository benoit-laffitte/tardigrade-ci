use serde::{Deserialize, Serialize};

/// Retry policy hook allowing DSL-level override per step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineRetryPolicy {
    pub max_attempts: u32,
    pub backoff_ms: u64,
}

impl PipelineRetryPolicy {
    /// Creates retry policy with max attempts and linear backoff in milliseconds.
    pub fn new(max_attempts: u32, backoff_ms: u64) -> Self {
        Self {
            max_attempts,
            backoff_ms,
        }
    }
}
