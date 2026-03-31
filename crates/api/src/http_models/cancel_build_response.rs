use serde::{Deserialize, Serialize};
use tardigrade_core::BuildRecord;

/// Response payload containing canceled build state.
#[derive(Debug, Serialize, Deserialize)]
pub struct CancelBuildResponse {
    pub build: BuildRecord,
}
