use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tardigrade_plugins::{
    Plugin, PluginCapability, PluginLifecycleError, PluginLifecycleState, PluginRegistry,
};

use crate::{PluginAuthorizationCheckResponse, PluginInfo, PluginPolicyResponse};

const GLOBAL_POLICY_CONTEXT: &str = "global";

/// Application use-case facade for plugin lifecycle and authorization policy operations.
#[derive(Clone)]
pub struct PluginUseCases {
    plugin_registry: Arc<Mutex<PluginRegistry>>,
    plugin_policy_store: Arc<Mutex<BTreeMap<String, Vec<PluginCapability>>>>,
}

impl PluginUseCases {
    /// Creates plugin use-cases with in-memory lifecycle registry and policy store.
    pub fn new() -> Self {
        Self {
            plugin_registry: Arc::new(Mutex::new(PluginRegistry::default())),
            plugin_policy_store: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Returns plugin inventory snapshot for administration panel.
    pub fn list_plugins(&self) -> Result<Vec<PluginInfo>, PluginLifecycleError> {
        let registry = self
            .plugin_registry
            .lock()
            .map_err(|_| PluginLifecycleError::ExecutionFailed)?;

        let plugins = registry
            .names()
            .into_iter()
            .filter_map(|name| plugin_info_from_registry(&registry, &name))
            .collect();

        Ok(plugins)
    }

    /// Loads one plugin from built-in catalog into lifecycle registry.
    pub fn load_plugin(&self, name: &str) -> Result<PluginInfo, PluginLifecycleError> {
        let plugin = create_builtin_plugin(name).ok_or(PluginLifecycleError::UnknownPlugin)?;
        let mut registry = self
            .plugin_registry
            .lock()
            .map_err(|_| PluginLifecycleError::ExecutionFailed)?;
        registry.load(plugin)?;
        plugin_info_from_registry(&registry, name).ok_or(PluginLifecycleError::NotFound)
    }

    /// Initializes one previously loaded plugin.
    pub fn init_plugin(&self, name: &str) -> Result<PluginInfo, PluginLifecycleError> {
        let mut registry = self
            .plugin_registry
            .lock()
            .map_err(|_| PluginLifecycleError::ExecutionFailed)?;
        registry.init(name)?;
        plugin_info_from_registry(&registry, name).ok_or(PluginLifecycleError::NotFound)
    }

    /// Executes one initialized plugin for diagnostics.
    pub fn execute_plugin(&self, name: &str) -> Result<PluginInfo, PluginLifecycleError> {
        let registry = self
            .plugin_registry
            .lock()
            .map_err(|_| PluginLifecycleError::ExecutionFailed)?;
        registry.execute(name)?;
        plugin_info_from_registry(&registry, name).ok_or(PluginLifecycleError::NotFound)
    }

    /// Unloads one plugin and marks it as no longer executable.
    pub fn unload_plugin(&self, name: &str) -> Result<PluginInfo, PluginLifecycleError> {
        let mut registry = self
            .plugin_registry
            .lock()
            .map_err(|_| PluginLifecycleError::ExecutionFailed)?;
        registry.unload(name)?;
        plugin_info_from_registry(&registry, name).ok_or(PluginLifecycleError::NotFound)
    }

    /// Upserts granted capabilities for one plugin policy context.
    pub fn upsert_plugin_policy(
        &self,
        context: Option<&str>,
        granted_capabilities: Vec<String>,
    ) -> Result<PluginPolicyResponse, PluginLifecycleError> {
        let normalized_context = normalize_policy_context(context);
        let parsed = parse_and_normalize_capabilities(&granted_capabilities)
            .map_err(|_| PluginLifecycleError::ExecutionFailed)?;

        let mut store = self
            .plugin_policy_store
            .lock()
            .map_err(|_| PluginLifecycleError::ExecutionFailed)?;
        store.insert(normalized_context.clone(), parsed.clone());

        Ok(PluginPolicyResponse {
            context: normalized_context,
            granted_capabilities: parsed
                .iter()
                .copied()
                .map(plugin_capability_as_str)
                .map(ToString::to_string)
                .collect(),
        })
    }

    /// Returns current granted capabilities for one context with global fallback.
    pub fn plugin_policy(
        &self,
        context: Option<&str>,
    ) -> Result<PluginPolicyResponse, PluginLifecycleError> {
        let normalized_context = normalize_policy_context(context);
        let store = self
            .plugin_policy_store
            .lock()
            .map_err(|_| PluginLifecycleError::ExecutionFailed)?;

        let granted = resolve_granted_capabilities(&store, &normalized_context);
        Ok(PluginPolicyResponse {
            context: normalized_context,
            granted_capabilities: granted
                .into_iter()
                .map(plugin_capability_as_str)
                .map(ToString::to_string)
                .collect(),
        })
    }

    /// Evaluates whether one plugin is authorized in one context and returns missing capabilities.
    pub fn plugin_authorization_check(
        &self,
        plugin_name: &str,
        context: Option<&str>,
    ) -> Result<PluginAuthorizationCheckResponse, PluginLifecycleError> {
        let normalized_context = normalize_policy_context(context);

        let registry = self
            .plugin_registry
            .lock()
            .map_err(|_| PluginLifecycleError::ExecutionFailed)?;
        let required = registry
            .capabilities(plugin_name)
            .ok_or(PluginLifecycleError::NotFound)?;
        drop(registry);

        let store = self
            .plugin_policy_store
            .lock()
            .map_err(|_| PluginLifecycleError::ExecutionFailed)?;
        let granted = resolve_granted_capabilities(&store, &normalized_context);

        let missing: Vec<PluginCapability> = required
            .iter()
            .copied()
            .filter(|capability| !granted.contains(capability))
            .collect();

        Ok(PluginAuthorizationCheckResponse {
            plugin_name: plugin_name.to_string(),
            context: normalized_context,
            required_capabilities: required
                .iter()
                .copied()
                .map(plugin_capability_as_str)
                .map(ToString::to_string)
                .collect(),
            granted_capabilities: granted
                .iter()
                .copied()
                .map(plugin_capability_as_str)
                .map(ToString::to_string)
                .collect(),
            missing_capabilities: missing
                .iter()
                .copied()
                .map(plugin_capability_as_str)
                .map(ToString::to_string)
                .collect(),
            allowed: missing.is_empty(),
        })
    }
}

impl Default for PluginUseCases {
    /// Builds default plugin use-case instance with in-memory state.
    fn default() -> Self {
        Self::new()
    }
}

/// Builds one API-friendly plugin inventory entry from registry internals.
fn plugin_info_from_registry(registry: &PluginRegistry, name: &str) -> Option<PluginInfo> {
    let state = registry.state(name)?;
    let capabilities = registry.capabilities(name)?;

    Some(PluginInfo {
        name: name.to_string(),
        state: plugin_state_as_str(state).to_string(),
        capabilities: capabilities
            .into_iter()
            .map(plugin_capability_as_str)
            .map(ToString::to_string)
            .collect(),
        source_manifest_entry: format!("builtin:{name}"),
    })
}

/// Maps plugin lifecycle enum to wire-format string.
fn plugin_state_as_str(state: PluginLifecycleState) -> &'static str {
    match state {
        PluginLifecycleState::Loaded => "Loaded",
        PluginLifecycleState::Initialized => "Initialized",
        PluginLifecycleState::Unloaded => "Unloaded",
    }
}

/// Maps capability enum to wire-format identifier used in UI.
fn plugin_capability_as_str(capability: PluginCapability) -> &'static str {
    match capability {
        PluginCapability::Network => "network",
        PluginCapability::Filesystem => "filesystem",
        PluginCapability::Secrets => "secrets",
        PluginCapability::RuntimeHooks => "runtime_hooks",
    }
}

/// Parses capability strings, validates them, and normalizes ordering/deduplication.
fn parse_and_normalize_capabilities(
    capabilities: &[String],
) -> Result<Vec<PluginCapability>, &'static str> {
    let mut parsed = Vec::with_capacity(capabilities.len());
    for capability in capabilities {
        let parsed_capability = match capability.trim().to_ascii_lowercase().as_str() {
            "network" => PluginCapability::Network,
            "filesystem" => PluginCapability::Filesystem,
            "secrets" => PluginCapability::Secrets,
            "runtime_hooks" => PluginCapability::RuntimeHooks,
            _ => return Err("unknown capability"),
        };
        parsed.push(parsed_capability);
    }

    parsed.sort();
    parsed.dedup();
    Ok(parsed)
}

/// Normalizes policy context and falls back to global context when absent.
fn normalize_policy_context(context: Option<&str>) -> String {
    let Some(value) = context else {
        return GLOBAL_POLICY_CONTEXT.to_string();
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return GLOBAL_POLICY_CONTEXT.to_string();
    }

    trimmed.to_string()
}

/// Resolves granted capabilities for context with fallback to global default context.
fn resolve_granted_capabilities(
    store: &BTreeMap<String, Vec<PluginCapability>>,
    context: &str,
) -> Vec<PluginCapability> {
    if let Some(capabilities) = store.get(context) {
        return capabilities.clone();
    }

    store
        .get(GLOBAL_POLICY_CONTEXT)
        .cloned()
        .unwrap_or_default()
}

/// Resolves one built-in plugin implementation by stable name.
fn create_builtin_plugin(name: &str) -> Option<Box<dyn Plugin>> {
    match name {
        "net-diagnostics" => Some(Box::new(NetworkDiagnosticsPlugin)),
        "fs-audit" => Some(Box::new(FilesystemAuditPlugin)),
        "panic-probe" => Some(Box::new(PanicProbePlugin)),
        _ => None,
    }
}

/// Built-in plugin used to validate network capability flow.
struct NetworkDiagnosticsPlugin;

impl Plugin for NetworkDiagnosticsPlugin {
    /// Returns stable plugin identifier used by API clients.
    fn name(&self) -> &'static str {
        "net-diagnostics"
    }

    /// Declares required permissions for this plugin implementation.
    fn required_capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Network]
    }
}

/// Built-in plugin used to validate filesystem capability flow.
struct FilesystemAuditPlugin;

impl Plugin for FilesystemAuditPlugin {
    /// Returns stable plugin identifier used by API clients.
    fn name(&self) -> &'static str {
        "fs-audit"
    }

    /// Declares required permissions for this plugin implementation.
    fn required_capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Filesystem]
    }
}

/// Built-in plugin used to exercise panic containment behavior.
struct PanicProbePlugin;

impl Plugin for PanicProbePlugin {
    /// Returns stable plugin identifier used by API clients.
    fn name(&self) -> &'static str {
        "panic-probe"
    }

    /// Declares required permissions for this plugin implementation.
    fn required_capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::RuntimeHooks]
    }

    /// Forces a panic to ensure runtime containment remains observable from API.
    fn on_execute(&self) -> Result<(), PluginLifecycleError> {
        panic!("panic-probe execution panic")
    }
}
