use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

/// Extension contract for runtime plugin modules.
pub trait Plugin: Send + Sync {
    /// Returns stable unique plugin name.
    fn name(&self) -> &'static str;
    /// Declares plugin capabilities required by implementation.
    fn required_capabilities(&self) -> Vec<PluginCapability> {
        Vec::new()
    }
    /// Optional hook called once when plugin is accepted by the registry.
    fn on_load(&self) {}
    /// Optional hook called when plugin is initialized before execution.
    fn on_init(&self) {}
    /// Optional hook called when plugin is executed by name.
    fn on_execute(&self) -> Result<(), PluginLifecycleError> {
        Ok(())
    }
    /// Optional hook called when plugin is unloaded from registry lifecycle.
    fn on_unload(&self) {}
}

/// Capability families used by plugin permissions and policy checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    Network,
    Filesystem,
    Secrets,
    RuntimeHooks,
}

/// Lifecycle state for one registered plugin instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginLifecycleState {
    Loaded,
    Initialized,
    Unloaded,
}

/// Error model for lifecycle operations in the plugin registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginLifecycleError {
    DuplicateName,
    NotFound,
    InvalidState,
    ExecutionFailed,
    ManifestIo,
    ManifestParse,
    UnknownPlugin,
}

/// Manifest root describing plugin discovery entries from filesystem.
#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub plugins: Vec<PluginManifestEntry>,
}

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

/// One plugin entry with runtime lifecycle state.
struct PluginEntry {
    plugin: Box<dyn Plugin>,
    state: PluginLifecycleState,
    capabilities: Vec<PluginCapability>,
}

/// In-memory plugin registry indexed by plugin name.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: BTreeMap<String, PluginEntry>,
}

impl PluginRegistry {
    /// Parses TOML manifest content into plugin discovery model.
    pub fn parse_manifest(contents: &str) -> Result<PluginManifest, PluginLifecycleError> {
        toml::from_str(contents).map_err(|_| PluginLifecycleError::ManifestParse)
    }

    /// Loads enabled plugins declared in filesystem manifest using caller-provided factory.
    pub fn load_from_manifest_path<F>(
        &mut self,
        path: impl AsRef<Path>,
        mut factory: F,
    ) -> Result<Vec<String>, PluginLifecycleError>
    where
        F: FnMut(&str) -> Option<Box<dyn Plugin>>,
    {
        let contents = fs::read_to_string(path).map_err(|_| PluginLifecycleError::ManifestIo)?;
        let manifest = Self::parse_manifest(&contents)?;

        let mut loaded = Vec::new();
        for entry in manifest.plugins {
            if !entry.enabled {
                continue;
            }

            let plugin = factory(&entry.name).ok_or(PluginLifecycleError::UnknownPlugin)?;
            let capabilities = if entry.capabilities.is_empty() {
                plugin.required_capabilities()
            } else {
                entry.capabilities
            };
            self.load_with_capabilities(plugin, capabilities)?;
            loaded.push(entry.name);
        }

        Ok(loaded)
    }

    /// Loads plugin into registry if name is unique.
    pub fn load(&mut self, plugin: Box<dyn Plugin>) -> Result<(), PluginLifecycleError> {
        let capabilities = plugin.required_capabilities();
        self.load_with_capabilities(plugin, capabilities)
    }

    /// Loads plugin into registry with explicit capability declaration.
    pub fn load_with_capabilities(
        &mut self,
        plugin: Box<dyn Plugin>,
        mut capabilities: Vec<PluginCapability>,
    ) -> Result<(), PluginLifecycleError> {
        let name = plugin.name().to_string();
        // Registry enforces unique plugin names to avoid ambiguous routing.
        if self.plugins.contains_key(&name) {
            return Err(PluginLifecycleError::DuplicateName);
        }

        // Normalizing here provides deterministic outputs for tests and policy layer.
        capabilities.sort();
        capabilities.dedup();

        plugin.on_load();
        self.plugins.insert(
            name,
            PluginEntry {
                plugin,
                state: PluginLifecycleState::Loaded,
                capabilities,
            },
        );
        Ok(())
    }

    /// Initializes a loaded plugin.
    pub fn init(&mut self, name: &str) -> Result<(), PluginLifecycleError> {
        let entry = self
            .plugins
            .get_mut(name)
            .ok_or(PluginLifecycleError::NotFound)?;

        match entry.state {
            PluginLifecycleState::Loaded => {
                entry.plugin.on_init();
                entry.state = PluginLifecycleState::Initialized;
                Ok(())
            }
            PluginLifecycleState::Initialized | PluginLifecycleState::Unloaded => {
                Err(PluginLifecycleError::InvalidState)
            }
        }
    }

    /// Executes an initialized plugin.
    pub fn execute(&self, name: &str) -> Result<(), PluginLifecycleError> {
        let entry = self
            .plugins
            .get(name)
            .ok_or(PluginLifecycleError::NotFound)?;

        if entry.state != PluginLifecycleState::Initialized {
            return Err(PluginLifecycleError::InvalidState);
        }

        entry
            .plugin
            .on_execute()
            .map_err(|_| PluginLifecycleError::ExecutionFailed)
    }

    /// Unloads a plugin and marks it as no longer executable.
    pub fn unload(&mut self, name: &str) -> Result<(), PluginLifecycleError> {
        let entry = self
            .plugins
            .get_mut(name)
            .ok_or(PluginLifecycleError::NotFound)?;

        match entry.state {
            PluginLifecycleState::Loaded | PluginLifecycleState::Initialized => {
                entry.plugin.on_unload();
                entry.state = PluginLifecycleState::Unloaded;
                Ok(())
            }
            PluginLifecycleState::Unloaded => Err(PluginLifecycleError::InvalidState),
        }
    }

    /// Returns current lifecycle state for one plugin, if known.
    pub fn state(&self, name: &str) -> Option<PluginLifecycleState> {
        self.plugins.get(name).map(|entry| entry.state)
    }

    /// Returns normalized capability list declared for one loaded plugin.
    pub fn capabilities(&self, name: &str) -> Option<Vec<PluginCapability>> {
        self.plugins.get(name).map(|entry| entry.capabilities.clone())
    }

    /// Backward-compatible alias for load returning boolean success.
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> bool {
        self.load(plugin).is_ok()
    }

    /// Returns number of registered plugins.
    pub fn count(&self) -> usize {
        self.plugins.len()
    }
}

#[cfg(test)]
mod tests;
