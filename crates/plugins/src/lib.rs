use std::collections::BTreeMap;

/// Extension contract for runtime plugin modules.
pub trait Plugin: Send + Sync {
    /// Returns stable unique plugin name.
    fn name(&self) -> &'static str;
    /// Optional hook called once when plugin is accepted by the registry.
    fn on_load(&self) {}
}

/// In-memory plugin registry indexed by plugin name.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: BTreeMap<String, Box<dyn Plugin>>,
}

impl PluginRegistry {
    /// Registers plugin if name is unique, returning true on success.
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> bool {
        let name = plugin.name().to_string();
        // Registry enforces unique plugin names to avoid ambiguous routing.
        if self.plugins.contains_key(&name) {
            return false;
        }

        plugin.on_load();
        self.plugins.insert(name, plugin);
        true
    }

    /// Returns number of registered plugins.
    pub fn count(&self) -> usize {
        self.plugins.len()
    }
}

#[cfg(test)]
mod tests;
