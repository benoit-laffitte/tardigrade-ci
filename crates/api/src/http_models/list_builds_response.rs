use serde::{Deserialize, Serialize};
use tardigrade_core::BuildRecord;

/// Response payload listing builds.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListBuildsResponse {
    pub builds: Vec<BuildRecord>,
}
