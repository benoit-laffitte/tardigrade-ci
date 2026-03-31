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
