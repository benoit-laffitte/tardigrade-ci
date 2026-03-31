use crate::{PluginCapability, PluginLifecycleError};

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
