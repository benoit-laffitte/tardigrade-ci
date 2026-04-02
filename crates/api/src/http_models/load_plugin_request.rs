use serde::{Deserialize, Serialize};

/// Request payload for loading one plugin by name.
#[derive(Debug, Serialize, Deserialize)]
pub struct LoadPluginRequest {
    pub name: String,
}
