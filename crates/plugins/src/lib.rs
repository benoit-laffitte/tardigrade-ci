mod contract;
mod error;
mod manifest;
mod model;
mod registry;

pub use contract::Plugin;
pub use error::PluginLifecycleError;
pub use manifest::{PluginManifest, PluginManifestEntry};
pub use model::{PluginCapability, PluginLifecycleState};
pub use registry::PluginRegistry;
