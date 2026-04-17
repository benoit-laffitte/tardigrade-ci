use serde::{Deserialize, Serialize};

/// Response payload describing plugin authorization decision and missing capabilities.
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginAuthorizationCheckResponse {
    pub plugin_name: String,
    pub context: String,
    pub required_capabilities: Vec<String>,
    pub granted_capabilities: Vec<String>,
    pub missing_capabilities: Vec<String>,
    pub allowed: bool,
}
