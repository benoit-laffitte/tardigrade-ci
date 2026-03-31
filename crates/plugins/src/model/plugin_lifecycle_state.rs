/// Lifecycle state for one registered plugin instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginLifecycleState {
    Loaded,
    Initialized,
    Unloaded,
}
