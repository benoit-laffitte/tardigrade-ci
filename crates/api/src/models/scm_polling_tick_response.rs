use serde::{Deserialize, Serialize};

/// Response payload for one SCM polling tick execution.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScmPollingTickResponse {
    pub polled_repositories: usize,
    pub enqueued_builds: usize,
}
