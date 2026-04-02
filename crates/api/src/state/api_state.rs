use axum::http::StatusCode;
use chrono::Utc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tardigrade_plugins::{
    Plugin, PluginCapability, PluginLifecycleError, PluginLifecycleState, PluginRegistry,
};
use tardigrade_core::{ScmPollingConfig, WebhookSecurityConfig};
use tardigrade_scheduler::{InMemoryScheduler, Scheduler};
use tardigrade_storage::{InMemoryStorage, Storage};

use crate::service::CiService;
use crate::{
    PluginInfo, ServiceSettings, UpsertScmPollingConfigRequest, UpsertWebhookSecurityConfigRequest,
};

/// Shared runtime state injected into HTTP and GraphQL handlers.
#[derive(Clone)]
pub struct ApiState {
    pub service_name: String,
    /// Service owns all domain orchestration (storage, scheduler, metrics, events).
    pub(crate) service: Arc<CiService>,
    /// Plugin registry stores loaded lifecycle state for administration endpoints.
    pub(crate) plugin_registry: Arc<Mutex<PluginRegistry>>,
    pub(crate) run_embedded_worker: bool,
}

impl ApiState {
    /// Builds default API state with in-memory storage and scheduler.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self::with_components(
            service_name,
            Arc::new(InMemoryStorage::default()),
            Arc::new(InMemoryScheduler::default()),
        )
    }

    /// Builds API state overriding storage backend while keeping in-memory scheduler.
    pub fn with_storage(
        service_name: impl Into<String>,
        storage: Arc<dyn Storage + Send + Sync>,
    ) -> Self {
        Self::with_components(service_name, storage, Arc::new(InMemoryScheduler::default()))
    }

    /// Builds API state from explicit storage and scheduler components.
    pub fn with_components(
        service_name: impl Into<String>,
        storage: Arc<dyn Storage + Send + Sync>,
        scheduler: Arc<dyn Scheduler + Send + Sync>,
    ) -> Self {
        Self::with_components_and_mode(service_name, storage, scheduler, true)
    }

    /// Builds API state and configures whether embedded worker loop is enabled.
    pub fn with_components_and_mode(
        service_name: impl Into<String>,
        storage: Arc<dyn Storage + Send + Sync>,
        scheduler: Arc<dyn Scheduler + Send + Sync>,
        run_embedded_worker: bool,
    ) -> Self {
        Self::with_components_and_mode_and_settings(
            service_name,
            storage,
            scheduler,
            run_embedded_worker,
            ServiceSettings::from_env(),
        )
    }

    /// Builds API state with explicit reliability settings (useful for deterministic tests).
    pub fn with_components_and_mode_and_settings(
        service_name: impl Into<String>,
        storage: Arc<dyn Storage + Send + Sync>,
        scheduler: Arc<dyn Scheduler + Send + Sync>,
        run_embedded_worker: bool,
        settings: ServiceSettings,
    ) -> Self {
        Self {
            service_name: service_name.into(),
            service: Arc::new(CiService::new(storage, scheduler, settings)),
            plugin_registry: Arc::new(Mutex::new(PluginRegistry::default())),
            run_embedded_worker,
        }
    }

    /// Returns plugin inventory snapshot for administration panel.
    pub fn list_plugins(&self) -> Result<Vec<PluginInfo>, StatusCode> {
        let registry = self
            .plugin_registry
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

    /// Upserts one repository-level webhook verification configuration.
    pub async fn upsert_webhook_security_config(
        &self,
        request: UpsertWebhookSecurityConfigRequest,
    ) -> Result<(), StatusCode> {
        if request.repository_url.trim().is_empty() || request.secret.trim().is_empty() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let config = WebhookSecurityConfig {
            repository_url: request.repository_url,
            provider: request.provider,
            secret: request.secret,
            allowed_ips: request.allowed_ips,
            updated_at: Utc::now(),
        };

        self.service
            .storage
            .upsert_webhook_security_config(config)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Upserts one SCM polling configuration for one repository/provider.
    pub async fn upsert_scm_polling_config(
        &self,
        request: UpsertScmPollingConfigRequest,
    ) -> Result<(), StatusCode> {
        if request.repository_url.trim().is_empty() || request.interval_secs == 0 {
            return Err(StatusCode::BAD_REQUEST);
        }

        let config = ScmPollingConfig {
            repository_url: request.repository_url,
            provider: request.provider,
            enabled: request.enabled,
            interval_secs: request.interval_secs,
            branches: request.branches,
            last_polled_at: None,
            updated_at: Utc::now(),
        };

        self.service
            .storage
            .upsert_scm_polling_config(config)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Starts SCM polling background loop with fixed check interval.
    pub fn start_scm_polling_loop(&self, check_interval: Duration) {
        let service = self.service.clone();
        tokio::spawn(async move {
            loop {
                let _ = service.run_scm_polling_tick().await;
                tokio::time::sleep(check_interval).await;
            }
        });
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
