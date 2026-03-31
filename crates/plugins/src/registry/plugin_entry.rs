use crate::{Plugin, PluginCapability, PluginLifecycleState};

/// One plugin entry with runtime lifecycle state.
pub(crate) struct PluginEntry {
    pub(crate) plugin: Box<dyn Plugin>,
    pub(crate) state: PluginLifecycleState,
    pub(crate) capabilities: Vec<PluginCapability>,
}
