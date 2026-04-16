mod contract;
mod error;
mod manifest;
mod model;
mod registry;

pub use self::{
    contract::Plugin,
    error::PluginLifecycleError,
    manifest::{PluginManifest, PluginManifestEntry},
    model::{PluginCapability, PluginLifecycleState},
    registry::PluginRegistry,
};
