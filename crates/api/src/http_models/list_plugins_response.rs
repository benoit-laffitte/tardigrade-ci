use serde::{Deserialize, Serialize};

use super::PluginInfo;

/// Response payload listing plugin registry inventory snapshot.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListPluginsResponse {
    pub plugins: Vec<PluginInfo>,
}
