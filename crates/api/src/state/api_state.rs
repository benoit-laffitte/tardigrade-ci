use axum::http::StatusCode;
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tardigrade_core::{ScmPollingConfig, WebhookSecurityConfig};
use tardigrade_scheduler::{InMemoryScheduler, Scheduler};
use tardigrade_storage::{InMemoryStorage, Storage};

use crate::service::CiService;
use crate::{ServiceSettings, UpsertScmPollingConfigRequest, UpsertWebhookSecurityConfigRequest};

/// Shared runtime state injected into HTTP and GraphQL handlers.
#[derive(Clone)]
pub struct ApiState {
    pub service_name: String,
    /// Service owns all domain orchestration (storage, scheduler, metrics, events).
    pub(crate) service: Arc<CiService>,
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
            run_embedded_worker,
        }
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
