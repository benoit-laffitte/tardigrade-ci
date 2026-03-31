use std::collections::BTreeMap;

/// Extension contract for runtime plugin modules.
pub trait Plugin: Send + Sync {
    /// Returns stable unique plugin name.
    fn name(&self) -> &'static str;
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
}

/// One plugin entry with runtime lifecycle state.
struct PluginEntry {
    plugin: Box<dyn Plugin>,
    state: PluginLifecycleState,
}

/// In-memory plugin registry indexed by plugin name.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: BTreeMap<String, PluginEntry>,
}

impl PluginRegistry {
    /// Loads plugin into registry if name is unique.
    pub fn load(&mut self, plugin: Box<dyn Plugin>) -> Result<(), PluginLifecycleError> {
        let name = plugin.name().to_string();
        // Registry enforces unique plugin names to avoid ambiguous routing.
        if self.plugins.contains_key(&name) {
            return Err(PluginLifecycleError::DuplicateName);
        }

        plugin.on_load();
        self.plugins.insert(
            name,
            PluginEntry {
                plugin,
                state: PluginLifecycleState::Loaded,
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
