use serde::Deserialize;

use super::PluginManifestEntry;

/// Manifest root describing plugin discovery entries from filesystem.
#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub plugins: Vec<PluginManifestEntry>,
}
