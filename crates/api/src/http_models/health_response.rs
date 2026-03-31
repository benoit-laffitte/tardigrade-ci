use serde::{Deserialize, Serialize};

/// Response body for service health endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: String,
}
