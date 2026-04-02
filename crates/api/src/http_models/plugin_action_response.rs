use serde::{Deserialize, Serialize};

use super::PluginInfo;

/// Response payload for one plugin lifecycle action result.
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginActionResponse {
    pub status: String,
    pub plugin: PluginInfo,
}
