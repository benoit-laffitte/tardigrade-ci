use serde::{Deserialize, Serialize};

/// Request payload used to set granted plugin capabilities for one execution context.
#[derive(Debug, Serialize, Deserialize)]
pub struct UpsertPluginPolicyRequest {
    pub context: Option<String>,
    pub granted_capabilities: Vec<String>,
}
