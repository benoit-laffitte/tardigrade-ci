use serde::{Deserialize, Serialize};
use tardigrade_core::BuildRecord;

/// Response payload containing updated build after completion.
#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteBuildResponse {
    pub build: BuildRecord,
}
