use serde::{Deserialize, Serialize};

/// Lifecycle status for a build execution in the CI control-plane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobStatus {
    /// Pending means queued but not yet claimed by a worker.
    Pending,
    /// Running means a worker currently owns and executes the build.
    Running,
    /// Success/Failed/Canceled are terminal states.
    Success,
    Failed,
    Canceled,
}
