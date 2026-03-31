use serde::{Deserialize, Serialize};

/// Response body for process liveness endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct LiveResponse {
    pub status: &'static str,
}
