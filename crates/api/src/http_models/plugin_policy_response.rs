use serde::{Deserialize, Serialize};

/// Response payload representing one plugin policy context and granted capability set.
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginPolicyResponse {
    pub context: String,
    pub granted_capabilities: Vec<String>,
}
