# Plugin Authoring and Permissions

This guide documents how to author plugins and how capability authorization is enforced in the current runtime registry.

## Scope

- Plugin lifecycle model: load, init, execute, unload.
- Manifest-driven discovery (`plugins.toml`).
- Capability declaration and authorization checks.
- Failure containment behavior during execution.

## Plugin Contract

Plugins implement the Rust trait exposed by the `tardigrade-plugins` crate:

- `name()`: stable unique plugin identity.
- `required_capabilities()`: capabilities required by this plugin.
- `on_load()`: hook executed on registry load.
- `on_init()`: hook executed before plugin can run.
- `on_execute()`: execution hook.
- `on_unload()`: hook executed on unload.

Minimal example:

```rust
use tardigrade_plugins::{Plugin, PluginCapability, PluginLifecycleError};

struct MetricsPlugin;

impl Plugin for MetricsPlugin {
    fn name(&self) -> &'static str {
        "metrics"
    }

    fn required_capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Network, PluginCapability::RuntimeHooks]
    }

    fn on_execute(&self) -> Result<(), PluginLifecycleError> {
        Ok(())
    }
}
```

## Manifest Discovery

Registry discovery reads a TOML file using this shape:

```toml
[[plugins]]
name = "metrics"
enabled = true
capabilities = ["network", "runtime_hooks"]

[[plugins]]
name = "artifact_store"
enabled = false
```

Rules:

- `enabled` defaults to `true`.
- If `capabilities` is omitted/empty, plugin `required_capabilities()` are used.
- If `capabilities` is present, the manifest value overrides plugin defaults.
- Unknown plugin names from manifest fail loading.

## Capability Model

Current capability families:

- `network`
- `filesystem`
- `secrets`
- `runtime_hooks`

Registry normalizes stored capability sets (sorted + deduplicated).

## Authorization Model

Use explicit authorization execution when calling plugins from policy-aware runtime code:

- `execute_authorized(name, granted_capabilities)`

Behavior:

- Plugin must be in `Initialized` state.
- Every required capability for the plugin must be present in `granted_capabilities`.
- Missing grant returns `UnauthorizedCapability(<capability>)`.

Compatibility path:

- `execute(name)` remains available and executes with the plugin's own declared capabilities.

## Failure Containment

Plugin execution is panic-safe:

- Panic inside `on_execute()` is caught.
- Panic is mapped to `ExecutionPanicked`.
- Other plugins remain executable after one plugin panic.

Non-panic execution errors from plugin hooks map to `ExecutionFailed`.

## Operational Error Semantics

Current lifecycle errors include:

- `DuplicateName`
- `NotFound`
- `InvalidState`
- `UnauthorizedCapability(...)`
- `ExecutionPanicked`
- `ExecutionFailed`
- `ManifestIo`
- `ManifestParse`
- `UnknownPlugin`

## Recommended Authoring Practices

- Keep `name()` stable over time.
- Declare minimum required capabilities.
- Keep `on_execute()` idempotent where possible.
- Return typed failures from `on_execute()` instead of panicking.
- Use `on_init()` for setup and fail fast if dependencies are unavailable.
