use serde::{Deserialize, Serialize};
use tardigrade_core::BuildRecord;

/// Response payload for worker claim call.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClaimBuildResponse {
    pub build: Option<BuildRecord>,
}
