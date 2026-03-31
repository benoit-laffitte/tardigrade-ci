use super::{Plugin, PluginLifecycleError, PluginLifecycleState, PluginRegistry};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// Test plugin tracking lifecycle hook invocation counts.
struct TestPlugin {
    name: &'static str,
    load_count: Arc<AtomicUsize>,
    init_count: Arc<AtomicUsize>,
    execute_count: Arc<AtomicUsize>,
    unload_count: Arc<AtomicUsize>,
    fail_execute: bool,
}

/// Lifecycle hook implementation used by registry tests.
impl Plugin for TestPlugin {
    /// Returns unique test plugin name.
    fn name(&self) -> &'static str {
        self.name
    }

    /// Increments load counter when plugin is loaded.
    fn on_load(&self) {
        self.load_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Increments init counter when plugin is initialized.
    fn on_init(&self) {
        self.init_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Increments execute counter and optionally fails execution.
    fn on_execute(&self) -> Result<(), PluginLifecycleError> {
        self.execute_count.fetch_add(1, Ordering::SeqCst);
        if self.fail_execute {
            return Err(PluginLifecycleError::ExecutionFailed);
        }
        Ok(())
    }

    /// Increments unload counter when plugin is unloaded.
    fn on_unload(&self) {
        self.unload_count.fetch_add(1, Ordering::SeqCst);
    }
}

/// Ensures backward-compatible register path still loads unique plugins once.
#[test]
fn register_accepts_unique_name_and_invokes_on_load_once() {
    let mut registry = PluginRegistry::default();
    let load_count = Arc::new(AtomicUsize::new(0));
    let init_count = Arc::new(AtomicUsize::new(0));
    let execute_count = Arc::new(AtomicUsize::new(0));
    let unload_count = Arc::new(AtomicUsize::new(0));

    let inserted = registry.register(Box::new(TestPlugin {
        name: "metrics",
        load_count: load_count.clone(),
        init_count: init_count.clone(),
        execute_count: execute_count.clone(),
        unload_count: unload_count.clone(),
        fail_execute: false,
    }));

    assert!(inserted);
    assert_eq!(registry.count(), 1);
    assert_eq!(load_count.load(Ordering::SeqCst), 1);
    assert_eq!(registry.state("metrics"), Some(PluginLifecycleState::Loaded));
}

/// Ensures duplicate plugin names are rejected and second load hook is not called.
#[test]
fn register_rejects_duplicate_name_without_second_on_load() {
    let mut registry = PluginRegistry::default();
    let first_count = Arc::new(AtomicUsize::new(0));
    let second_count = Arc::new(AtomicUsize::new(0));
    let init_count = Arc::new(AtomicUsize::new(0));
    let execute_count = Arc::new(AtomicUsize::new(0));
    let unload_count = Arc::new(AtomicUsize::new(0));

    let first_insert = registry.register(Box::new(TestPlugin {
        name: "artifact-store",
        load_count: first_count.clone(),
        init_count: init_count.clone(),
        execute_count: execute_count.clone(),
        unload_count: unload_count.clone(),
        fail_execute: false,
    }));
    let second_insert = registry.register(Box::new(TestPlugin {
        name: "artifact-store",
        load_count: second_count.clone(),
        init_count: init_count.clone(),
        execute_count: execute_count.clone(),
        unload_count: unload_count.clone(),
        fail_execute: false,
    }));

    assert!(first_insert);
    assert!(!second_insert);
    assert_eq!(registry.count(), 1);
    assert_eq!(first_count.load(Ordering::SeqCst), 1);
    assert_eq!(second_count.load(Ordering::SeqCst), 0);
}

/// Validates explicit lifecycle path load -> init -> execute -> unload.
#[test]
fn lifecycle_transitions_succeed_in_order() {
    let mut registry = PluginRegistry::default();
    let load_count = Arc::new(AtomicUsize::new(0));
    let init_count = Arc::new(AtomicUsize::new(0));
    let execute_count = Arc::new(AtomicUsize::new(0));
    let unload_count = Arc::new(AtomicUsize::new(0));

    registry
        .load(Box::new(TestPlugin {
            name: "executor",
            load_count: load_count.clone(),
            init_count: init_count.clone(),
            execute_count: execute_count.clone(),
            unload_count: unload_count.clone(),
            fail_execute: false,
        }))
        .expect("load should succeed");
    assert_eq!(registry.state("executor"), Some(PluginLifecycleState::Loaded));

    registry.init("executor").expect("init should succeed");
    assert_eq!(registry.state("executor"), Some(PluginLifecycleState::Initialized));

    registry
        .execute("executor")
        .expect("execute should succeed");

    registry
        .unload("executor")
        .expect("unload should succeed");
    assert_eq!(registry.state("executor"), Some(PluginLifecycleState::Unloaded));

    assert_eq!(load_count.load(Ordering::SeqCst), 1);
    assert_eq!(init_count.load(Ordering::SeqCst), 1);
    assert_eq!(execute_count.load(Ordering::SeqCst), 1);
    assert_eq!(unload_count.load(Ordering::SeqCst), 1);
}

/// Ensures invalid lifecycle order is rejected for execute before init.
#[test]
fn execute_before_init_is_rejected() {
    let mut registry = PluginRegistry::default();
    let count = Arc::new(AtomicUsize::new(0));

    registry
        .load(Box::new(TestPlugin {
            name: "guard",
            load_count: count.clone(),
            init_count: count.clone(),
            execute_count: count.clone(),
            unload_count: count.clone(),
            fail_execute: false,
        }))
        .expect("load should succeed");

    let err = registry
        .execute("guard")
        .expect_err("execute before init should fail");
    assert_eq!(err, PluginLifecycleError::InvalidState);
}

/// Ensures execution failures from plugin hooks are surfaced as lifecycle errors.
#[test]
fn execute_failure_is_reported() {
    let mut registry = PluginRegistry::default();
    let count = Arc::new(AtomicUsize::new(0));

    registry
        .load(Box::new(TestPlugin {
            name: "failing-plugin",
            load_count: count.clone(),
            init_count: count.clone(),
            execute_count: count.clone(),
            unload_count: count.clone(),
            fail_execute: true,
        }))
        .expect("load should succeed");
    registry
        .init("failing-plugin")
        .expect("init should succeed");

    let err = registry
        .execute("failing-plugin")
        .expect_err("execution should fail");
    assert_eq!(err, PluginLifecycleError::ExecutionFailed);
}
