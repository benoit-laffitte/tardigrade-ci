mod plugin_entry;
mod plugin_registry;

pub(crate) use plugin_entry::PluginEntry;
pub use plugin_registry::PluginRegistry;

#[cfg(test)]
mod tests;
