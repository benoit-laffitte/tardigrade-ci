use axum::{
    Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tardigrade_application::{
    CiService, CiUseCases, PluginUseCases, ScmWebhookRequest, ServiceSettings,
};
use tardigrade_core::{ScmPollingConfig, WebhookSecurityConfig};
use tardigrade_plugins::PluginLifecycleError;
use tardigrade_scheduler::ports::Scheduler;
use tardigrade_storage::ports::Storage;

use crate::{
    ApiErrorResponse, PluginAuthorizationCheckResponse, PluginInfo, PluginPolicyResponse,
    ScmWebhookAcceptedResponse, UpsertScmPollingConfigRequest, UpsertWebhookSecurityConfigRequest,
};

/// Shared runtime state injected into HTTP and GraphQL handlers.
#[derive(Clone)]
pub struct ApiState {
    pub service_name: String,
    /// Use-case layer consumed by HTTP/GraphQL adapters.
    pub(crate) use_cases: Arc<CiUseCases>,
    /// Plugin administration use-cases consumed by GraphQL adapters.
    pub(crate) plugin_use_cases: Arc<PluginUseCases>,
}

impl ApiState {
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
        let service = Arc::new(CiService::new(storage, scheduler, settings));
        Self {
            service_name: service_name.into(),
            use_cases: Arc::new(CiUseCases::new(service)),
            plugin_use_cases: Arc::new(PluginUseCases::new()),
        }
    }

    /// Returns plugin inventory snapshot for administration panel.
    pub fn list_plugins(&self) -> Result<Vec<PluginInfo>, StatusCode> {
        self.plugin_use_cases
            .list_plugins()
            .map_err(map_plugin_err_to_status)
    }

    /// Loads one plugin from built-in catalog into lifecycle registry.
    pub fn load_plugin(&self, name: &str) -> Result<PluginInfo, PluginLifecycleError> {
        self.plugin_use_cases.load_plugin(name)
    }

    /// Initializes one previously loaded plugin.
    pub fn init_plugin(&self, name: &str) -> Result<PluginInfo, PluginLifecycleError> {
        self.plugin_use_cases.init_plugin(name)
    }

    /// Executes one initialized plugin for diagnostics.
    pub fn execute_plugin(&self, name: &str) -> Result<PluginInfo, PluginLifecycleError> {
        self.plugin_use_cases.execute_plugin(name)
    }

    /// Unloads one plugin and marks it as no longer executable.
    pub fn unload_plugin(&self, name: &str) -> Result<PluginInfo, PluginLifecycleError> {
        self.plugin_use_cases.unload_plugin(name)
    }

    /// Upserts granted capabilities for one plugin policy context.
    pub fn upsert_plugin_policy(
        &self,
        context: Option<&str>,
        granted_capabilities: Vec<String>,
    ) -> Result<PluginPolicyResponse, StatusCode> {
        self.plugin_use_cases
            .upsert_plugin_policy(context, granted_capabilities)
            .map_err(map_plugin_err_to_status)
    }

    /// Returns current granted capabilities for one context with global fallback.
    pub fn plugin_policy(&self, context: Option<&str>) -> Result<PluginPolicyResponse, StatusCode> {
        self.plugin_use_cases
            .plugin_policy(context)
            .map_err(map_plugin_err_to_status)
    }

    /// Evaluates whether one plugin is authorized in one context and returns missing capabilities.
    pub fn plugin_authorization_check(
        &self,
        plugin_name: &str,
        context: Option<&str>,
    ) -> Result<PluginAuthorizationCheckResponse, StatusCode> {
        self.plugin_use_cases
            .plugin_authorization_check(plugin_name, context)
            .map_err(map_plugin_err_to_status)
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

        self.use_cases
            .upsert_webhook_security_config(config)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Accepts one native SCM webhook over HTTP without restoring the general REST API surface.
    pub async fn ingest_scm_webhook_http(&self, headers: HeaderMap, body: &[u8]) -> Response {
        let request = webhook_request_from_http_headers(&headers, body);
        match self.use_cases.ingest_scm_webhook_observed(&request).await {
            Ok(()) => (
                StatusCode::ACCEPTED,
                Json(ScmWebhookAcceptedResponse {
                    status: "accepted".to_string(),
                }),
            )
                .into_response(),
            Err(failure) => {
                if let Some(message) = failure.public_message {
                    return (
                        failure.api_error.status_code(),
                        Json(ApiErrorResponse {
                            code: failure.reason_code.to_string(),
                            message: message.to_string(),
                            details: None,
                        }),
                    )
                        .into_response();
                }

                failure.api_error.status_code().into_response()
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

        self.use_cases
            .upsert_scm_polling_config(config)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Starts SCM polling background loop with fixed check interval.
    pub fn start_scm_polling_loop(&self, check_interval: Duration) {
        self.use_cases.start_scm_polling_loop(check_interval);
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

/// Converts plugin use-case failures into stable HTTP statuses for adapters.
fn map_plugin_err_to_status(error: PluginLifecycleError) -> StatusCode {
    match error {
        PluginLifecycleError::UnknownPlugin | PluginLifecycleError::NotFound => {
            StatusCode::NOT_FOUND
        }
        PluginLifecycleError::DuplicateName => StatusCode::CONFLICT,
        PluginLifecycleError::InvalidState
        | PluginLifecycleError::UnauthorizedCapability(_)
        | PluginLifecycleError::ManifestIo
        | PluginLifecycleError::ManifestParse
        | PluginLifecycleError::ExecutionFailed
        | PluginLifecycleError::ExecutionPanicked => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
