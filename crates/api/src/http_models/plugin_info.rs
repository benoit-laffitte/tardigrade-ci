use serde::{Deserialize, Serialize};

/// One plugin entry returned by plugin administration endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub state: String,
    pub capabilities: Vec<String>,
    pub source_manifest_entry: String,
}
