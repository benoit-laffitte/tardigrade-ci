use serde::{Deserialize, Serialize};

/// Response body for readiness endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadyResponse {
    pub status: &'static str,
}
