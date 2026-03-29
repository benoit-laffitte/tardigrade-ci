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
