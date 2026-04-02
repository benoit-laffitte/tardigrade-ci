use serde::{Deserialize, Serialize};

/// Request payload used to evaluate plugin authorization in one execution context.
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginAuthorizationCheckRequest {
    pub context: Option<String>,
}
