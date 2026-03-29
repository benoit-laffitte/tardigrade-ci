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
mod tests {
    use super::{Plugin, PluginRegistry};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct TestPlugin {
        name: &'static str,
        load_count: Arc<AtomicUsize>,
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &'static str {
            self.name
        }

        fn on_load(&self) {
            self.load_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn register_accepts_unique_name_and_invokes_on_load_once() {
        let mut registry = PluginRegistry::default();
        let load_count = Arc::new(AtomicUsize::new(0));

        let inserted = registry.register(Box::new(TestPlugin {
            name: "metrics",
            load_count: load_count.clone(),
        }));

        assert!(inserted);
        assert_eq!(registry.count(), 1);
        assert_eq!(load_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn register_rejects_duplicate_name_without_second_on_load() {
        let mut registry = PluginRegistry::default();
        let first_count = Arc::new(AtomicUsize::new(0));
        let second_count = Arc::new(AtomicUsize::new(0));

        let first_insert = registry.register(Box::new(TestPlugin {
            name: "artifact-store",
            load_count: first_count.clone(),
        }));
        let second_insert = registry.register(Box::new(TestPlugin {
            name: "artifact-store",
            load_count: second_count.clone(),
        }));

        assert!(first_insert);
        assert!(!second_insert);
        assert_eq!(registry.count(), 1);
        assert_eq!(first_count.load(Ordering::SeqCst), 1);
        assert_eq!(second_count.load(Ordering::SeqCst), 0);
    }
}
