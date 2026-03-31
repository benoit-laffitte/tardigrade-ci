use serde::Deserialize;

use crate::PluginCapability;

/// One declared plugin entry in the filesystem manifest.
#[derive(Debug, Deserialize)]
pub struct PluginManifestEntry {
    pub name: String,
    #[serde(default = "manifest_enabled_default")]
    pub enabled: bool,
    #[serde(default)]
    pub capabilities: Vec<PluginCapability>,
}

/// Returns default enabled status for manifest entries.
fn manifest_enabled_default() -> bool {
    true
}
