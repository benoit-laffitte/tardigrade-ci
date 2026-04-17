use axum::{
    Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tardigrade_core::{ScmPollingConfig, WebhookSecurityConfig};
use tardigrade_plugins::{
    Plugin, PluginCapability, PluginLifecycleError, PluginLifecycleState, PluginRegistry,
};
use tardigrade_scheduler::{InMemoryScheduler, Scheduler};
use tardigrade_storage::{InMemoryStorage, Storage};

use crate::service::{CiService, ScmWebhookRequest};
use crate::{
    ApiError, ApiErrorResponse, PluginAuthorizationCheckResponse, PluginInfo, PluginPolicyResponse,
    ScmWebhookAcceptedResponse, ServiceSettings, UpsertScmPollingConfigRequest,
    UpsertWebhookSecurityConfigRequest,
};

const GLOBAL_POLICY_CONTEXT: &str = "global";

/// Shared runtime state injected into HTTP and GraphQL handlers.
#[derive(Clone)]
pub struct ApiState {
    pub service_name: String,
    /// Service owns all domain orchestration (storage, scheduler, metrics, events).
    pub(crate) service: Arc<CiService>,
    /// Plugin registry stores loaded lifecycle state for administration endpoints.
    pub(crate) plugin_registry: Arc<Mutex<PluginRegistry>>,
    /// Plugin policy store maps execution context to granted capabilities.
    pub(crate) plugin_policy_store: Arc<Mutex<BTreeMap<String, Vec<PluginCapability>>>>,
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
        Self::with_components(
            service_name,
            storage,
            Arc::new(InMemoryScheduler::default()),
        )
    }

    /// Builds API state from explicit storage and scheduler components.
    pub fn with_components(
        service_name: impl Into<String>,
        storage: Arc<dyn Storage + Send + Sync>,
        scheduler: Arc<dyn Scheduler + Send + Sync>,
    ) -> Self {
        Self::with_components_and_settings(
            service_name,
            storage,
            scheduler,
            ServiceSettings::default(),
        )
    }

    /// Builds API state with explicit reliability settings (useful for deterministic tests).
    pub fn with_components_and_settings(
        service_name: impl Into<String>,
        storage: Arc<dyn Storage + Send + Sync>,
        scheduler: Arc<dyn Scheduler + Send + Sync>,
        settings: ServiceSettings,
    ) -> Self {
        Self {
            service_name: service_name.into(),
            service: Arc::new(CiService::new(storage, scheduler, settings)),
            plugin_registry: Arc::new(Mutex::new(PluginRegistry::default())),
            plugin_policy_store: Arc::new(Mutex::new(BTreeMap::new())),
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

    /// Upserts granted capabilities for one plugin policy context.
    pub fn upsert_plugin_policy(
        &self,
        context: Option<&str>,
        granted_capabilities: Vec<String>,
    ) -> Result<PluginPolicyResponse, StatusCode> {
        let normalized_context = normalize_policy_context(context);
        let parsed = parse_and_normalize_capabilities(&granted_capabilities)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let mut store = self
            .plugin_policy_store
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    pub fn plugin_policy(&self, context: Option<&str>) -> Result<PluginPolicyResponse, StatusCode> {
        let normalized_context = normalize_policy_context(context);
        let store = self
            .plugin_policy_store
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    ) -> Result<PluginAuthorizationCheckResponse, StatusCode> {
        let normalized_context = normalize_policy_context(context);

        let registry = self
            .plugin_registry
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let required = registry
            .capabilities(plugin_name)
            .ok_or(StatusCode::NOT_FOUND)?;
        drop(registry);

        let store = self
            .plugin_policy_store
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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

    /// Accepts one native SCM webhook over HTTP without restoring the general REST API surface.
    pub async fn ingest_scm_webhook_http(&self, headers: HeaderMap, body: &[u8]) -> Response {
        let request = webhook_request_from_http_headers(&headers, body);
        let provider = request
            .header_value("x-scm-provider")
            .map(ToString::to_string);
        let repository_url = request
            .header_value("x-scm-repository")
            .map(ToString::to_string);

        self.service.record_scm_webhook_received();

        match self.service.ingest_scm_webhook(&request).await {
            Ok(()) => {
                self.service.record_scm_webhook_accepted();
                (
                    StatusCode::ACCEPTED,
                    Json(ScmWebhookAcceptedResponse {
                        status: "accepted".to_string(),
                    }),
                )
                    .into_response()
            }
            Err(ApiError::BadRequest) => {
                self.service.record_scm_webhook_rejected();
                self.service.record_scm_webhook_rejection(
                    "invalid_webhook_request",
                    provider.as_deref(),
                    repository_url.as_deref(),
                );
                (
                    StatusCode::BAD_REQUEST,
                    Json(ApiErrorResponse {
                        code: "invalid_webhook_request".to_string(),
                        message: "webhook request is missing required headers".to_string(),
                        details: None,
                    }),
                )
                    .into_response()
            }
            Err(ApiError::Unauthorized) => {
                self.service.record_scm_webhook_rejected();
                self.service.record_scm_webhook_rejection(
                    "invalid_webhook_signature",
                    provider.as_deref(),
                    repository_url.as_deref(),
                );
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiErrorResponse {
                        code: "invalid_webhook_signature".to_string(),
                        message: "webhook signature is missing, invalid, or expired".to_string(),
                        details: None,
                    }),
                )
                    .into_response()
            }
            Err(ApiError::Forbidden) => {
                self.service.record_scm_webhook_rejected();
                self.service.record_scm_webhook_rejection(
                    "webhook_forbidden",
                    provider.as_deref(),
                    repository_url.as_deref(),
                );
                (
                    StatusCode::FORBIDDEN,
                    Json(ApiErrorResponse {
                        code: "webhook_forbidden".to_string(),
                        message: "webhook provider/repository/ip is not authorized".to_string(),
                        details: None,
                    }),
                )
                    .into_response()
            }
            Err(err) => {
                self.service.record_scm_webhook_rejected();
                self.service.record_scm_webhook_rejection(
                    "webhook_internal_error",
                    provider.as_deref(),
                    repository_url.as_deref(),
                );
                err.status_code().into_response()
            }
        }
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

/// Converts HTTP-native headers/body into transport-neutral webhook command input.
fn webhook_request_from_http_headers(headers: &HeaderMap, body: &[u8]) -> ScmWebhookRequest {
    let header_pairs = headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|raw| (name.as_str().to_string(), raw.to_string()))
        })
        .collect::<Vec<_>>();

    ScmWebhookRequest::from_parts(header_pairs, body.to_vec())
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
