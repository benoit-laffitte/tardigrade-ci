use crate::{
    Plugin, PluginCapability, PluginLifecycleError, PluginLifecycleState, PluginRegistry,
};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

/// Test plugin tracking lifecycle hook invocation counts.
struct TestPlugin {
    name: &'static str,
    load_count: Arc<AtomicUsize>,
    init_count: Arc<AtomicUsize>,
    execute_count: Arc<AtomicUsize>,
    unload_count: Arc<AtomicUsize>,
    fail_execute: bool,
    panic_execute: bool,
    required_capabilities: Vec<PluginCapability>,
}

/// Lifecycle hook implementation used by registry tests.
impl Plugin for TestPlugin {
    /// Returns unique test plugin name.
    fn name(&self) -> &'static str {
        self.name
    }

    /// Returns required capabilities declared by test instance.
    fn required_capabilities(&self) -> Vec<PluginCapability> {
        self.required_capabilities.clone()
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
        if self.panic_execute {
            panic!("test panic from plugin execution");
        }
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
        panic_execute: false,
        required_capabilities: vec![],
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
        panic_execute: false,
        required_capabilities: vec![],
    }));
    let second_insert = registry.register(Box::new(TestPlugin {
        name: "artifact-store",
        load_count: second_count.clone(),
        init_count: init_count.clone(),
        execute_count: execute_count.clone(),
        unload_count: unload_count.clone(),
        fail_execute: false,
        panic_execute: false,
        required_capabilities: vec![],
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
            panic_execute: false,
            required_capabilities: vec![],
        }))
        .expect("load should succeed");
    assert_eq!(registry.state("executor"), Some(PluginLifecycleState::Loaded));

    registry.init("executor").expect("init should succeed");
    assert_eq!(registry.state("executor"), Some(PluginLifecycleState::Initialized));

    registry.execute("executor").expect("execute should succeed");

    registry.unload("executor").expect("unload should succeed");
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
            panic_execute: false,
            required_capabilities: vec![],
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
            panic_execute: false,
            required_capabilities: vec![],
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

/// Creates a temporary manifest file with provided content for filesystem loading tests.
fn write_temp_manifest(contents: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("tardigrade-plugins-test-{unique}"));
    std::fs::create_dir_all(&dir).expect("temp dir should be created");
    let manifest_path = dir.join("plugins.toml");
    std::fs::write(&manifest_path, contents).expect("manifest should be written");
    manifest_path
}

/// Verifies parser accepts valid manifest and respects default enabled value.
#[test]
fn parse_manifest_parses_enabled_and_default_values() {
    let parsed = PluginRegistry::parse_manifest(
        r#"
            [[plugins]]
            name = "metrics"
            capabilities = ["network", "runtime_hooks"]

            [[plugins]]
            name = "artifact-store"
            enabled = false
        "#,
    )
    .expect("manifest should parse");

    assert_eq!(parsed.plugins.len(), 2);
    assert_eq!(parsed.plugins[0].name, "metrics");
    assert!(parsed.plugins[0].enabled);
    assert_eq!(
        parsed.plugins[0].capabilities,
        vec![PluginCapability::Network, PluginCapability::RuntimeHooks]
    );
    assert_eq!(parsed.plugins[1].name, "artifact-store");
    assert!(!parsed.plugins[1].enabled);
}

/// Verifies loader reads filesystem manifest and loads only enabled known plugins.
#[test]
fn load_from_manifest_path_loads_enabled_entries() {
    let manifest_path = write_temp_manifest(
        r#"
            [[plugins]]
            name = "metrics"
            capabilities = ["network", "secrets"]

            [[plugins]]
            name = "artifact-store"
            enabled = false
        "#,
    );

    let mut registry = PluginRegistry::default();
    let load_count = Arc::new(AtomicUsize::new(0));
    let init_count = Arc::new(AtomicUsize::new(0));
    let execute_count = Arc::new(AtomicUsize::new(0));
    let unload_count = Arc::new(AtomicUsize::new(0));

    let loaded = registry
        .load_from_manifest_path(manifest_path, |name| match name {
            "metrics" => Some(Box::new(TestPlugin {
                name: "metrics",
                load_count: load_count.clone(),
                init_count: init_count.clone(),
                execute_count: execute_count.clone(),
                unload_count: unload_count.clone(),
                fail_execute: false,
                panic_execute: false,
                required_capabilities: vec![PluginCapability::Filesystem],
            })),
            _ => None,
        })
        .expect("manifest load should succeed");

    assert_eq!(loaded, vec!["metrics".to_string()]);
    assert_eq!(registry.count(), 1);
    assert_eq!(registry.state("metrics"), Some(PluginLifecycleState::Loaded));
    assert_eq!(
        registry.capabilities("metrics"),
        Some(vec![PluginCapability::Network, PluginCapability::Secrets])
    );
    assert_eq!(load_count.load(Ordering::SeqCst), 1);
}

/// Verifies loader fails when manifest references an unknown plugin name.
#[test]
fn load_from_manifest_path_fails_for_unknown_plugin() {
    let manifest_path = write_temp_manifest(
        r#"
            [[plugins]]
            name = "unknown-plugin"
        "#,
    );

    let mut registry = PluginRegistry::default();
    let err = registry
        .load_from_manifest_path(manifest_path, |_name| None)
        .expect_err("unknown plugin should fail loading");

    assert_eq!(err, PluginLifecycleError::UnknownPlugin);
}

/// Verifies direct load uses plugin-declared required capability model.
#[test]
fn load_uses_plugin_required_capabilities() {
    let mut registry = PluginRegistry::default();
    let count = Arc::new(AtomicUsize::new(0));

    registry
        .load(Box::new(TestPlugin {
            name: "policy-aware",
            load_count: count.clone(),
            init_count: count.clone(),
            execute_count: count.clone(),
            unload_count: count.clone(),
            fail_execute: false,
            panic_execute: false,
            required_capabilities: vec![
                PluginCapability::RuntimeHooks,
                PluginCapability::Network,
                PluginCapability::Network,
            ],
        }))
        .expect("load should succeed");

    assert_eq!(
        registry.capabilities("policy-aware"),
        Some(vec![
            PluginCapability::Network,
            PluginCapability::RuntimeHooks,
        ])
    );
}

/// Verifies explicit execution authorization rejects missing required capability grants.
#[test]
fn execute_authorized_rejects_missing_required_capability() {
    let mut registry = PluginRegistry::default();
    let count = Arc::new(AtomicUsize::new(0));

    registry
        .load(Box::new(TestPlugin {
            name: "secrets-plugin",
            load_count: count.clone(),
            init_count: count.clone(),
            execute_count: count.clone(),
            unload_count: count.clone(),
            fail_execute: false,
            panic_execute: false,
            required_capabilities: vec![PluginCapability::Secrets],
        }))
        .expect("load should succeed");
    registry
        .init("secrets-plugin")
        .expect("init should succeed");

    let err = registry
        .execute_authorized("secrets-plugin", &[PluginCapability::Network])
        .expect_err("missing secrets capability should be denied");

    assert_eq!(err, PluginLifecycleError::UnauthorizedCapability(PluginCapability::Secrets));
}

/// Verifies explicit execution authorization succeeds when all required capabilities are granted.
#[test]
fn execute_authorized_accepts_when_all_required_capabilities_are_granted() {
    let mut registry = PluginRegistry::default();
    let execute_count = Arc::new(AtomicUsize::new(0));
    let count = Arc::new(AtomicUsize::new(0));

    registry
        .load(Box::new(TestPlugin {
            name: "runtime-hook-plugin",
            load_count: count.clone(),
            init_count: count.clone(),
            execute_count: execute_count.clone(),
            unload_count: count.clone(),
            fail_execute: false,
            panic_execute: false,
            required_capabilities: vec![
                PluginCapability::RuntimeHooks,
                PluginCapability::Network,
            ],
        }))
        .expect("load should succeed");
    registry
        .init("runtime-hook-plugin")
        .expect("init should succeed");

    registry
        .execute_authorized(
            "runtime-hook-plugin",
            &[PluginCapability::Network, PluginCapability::RuntimeHooks],
        )
        .expect("authorized execution should succeed");

    assert_eq!(execute_count.load(Ordering::SeqCst), 1);
}

/// Verifies execution panic is contained and surfaced as typed lifecycle error.
#[test]
fn execute_panicked_plugin_is_reported_without_process_crash() {
    let mut registry = PluginRegistry::default();
    let count = Arc::new(AtomicUsize::new(0));

    registry
        .load(Box::new(TestPlugin {
            name: "panic-plugin",
            load_count: count.clone(),
            init_count: count.clone(),
            execute_count: count.clone(),
            unload_count: count.clone(),
            fail_execute: false,
            panic_execute: true,
            required_capabilities: vec![],
        }))
        .expect("load should succeed");
    registry.init("panic-plugin").expect("init should succeed");

    let err = registry
        .execute("panic-plugin")
        .expect_err("panic should be contained and mapped");
    assert_eq!(err, PluginLifecycleError::ExecutionPanicked);
}

/// Verifies one plugin panic does not block later execution of healthy plugins.
#[test]
fn panic_in_one_plugin_does_not_block_other_plugins() {
    let mut registry = PluginRegistry::default();
    let panic_count = Arc::new(AtomicUsize::new(0));
    let healthy_exec_count = Arc::new(AtomicUsize::new(0));
    let shared_count = Arc::new(AtomicUsize::new(0));

    registry
        .load(Box::new(TestPlugin {
            name: "panic-plugin",
            load_count: shared_count.clone(),
            init_count: shared_count.clone(),
            execute_count: panic_count.clone(),
            unload_count: shared_count.clone(),
            fail_execute: false,
            panic_execute: true,
            required_capabilities: vec![],
        }))
        .expect("panic plugin load should succeed");
    registry
        .load(Box::new(TestPlugin {
            name: "healthy-plugin",
            load_count: shared_count.clone(),
            init_count: shared_count.clone(),
            execute_count: healthy_exec_count.clone(),
            unload_count: shared_count.clone(),
            fail_execute: false,
            panic_execute: false,
            required_capabilities: vec![],
        }))
        .expect("healthy plugin load should succeed");

    registry
        .init("panic-plugin")
        .expect("panic plugin init should succeed");
    registry
        .init("healthy-plugin")
        .expect("healthy plugin init should succeed");

    let panic_err = registry
        .execute("panic-plugin")
        .expect_err("panic plugin should fail with panic-mapped error");
    assert_eq!(panic_err, PluginLifecycleError::ExecutionPanicked);

    registry
        .execute("healthy-plugin")
        .expect("healthy plugin should still execute after panic in another plugin");
    assert_eq!(healthy_exec_count.load(Ordering::SeqCst), 1);
}
