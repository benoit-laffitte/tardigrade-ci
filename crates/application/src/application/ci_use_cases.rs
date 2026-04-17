use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tardigrade_core::{
    BuildRecord, JobDefinition, ScmPollingConfig, WebhookSecurityConfig, WorkerBuildStatus,
};
use uuid::Uuid;

use crate::service::{ApiError, CiService, ScmWebhookRequest};
use crate::{
    CreateJobRequest, RuntimeMetricsResponse, ScmPollingTickResponse, ScmWebhookIngestFailure,
    ScmWebhookRejectionEntry, WorkerInfo,
};

/// Application use-case facade consumed by HTTP and GraphQL adapters.
#[derive(Clone)]
pub struct CiUseCases {
    service: Arc<CiService>,
}

impl CiUseCases {
    /// Creates use-case facade from one orchestrator service instance.
    pub fn new(service: Arc<CiService>) -> Self {
        Self { service }
    }

    /// Creates a job definition after validation.
    pub async fn create_job(&self, payload: CreateJobRequest) -> Result<JobDefinition, ApiError> {
        self.service.create_job(payload).await
    }

    /// Enqueues a build for one existing job.
    pub async fn run_job(&self, job_id: Uuid) -> Result<BuildRecord, ApiError> {
        self.service.run_job(job_id).await
    }

    /// Cancels one build and persists resulting state.
    pub async fn cancel_build(&self, build_id: Uuid) -> Result<BuildRecord, ApiError> {
        self.service.cancel_build(build_id).await
    }

    /// Claims next build for one worker identity.
    pub async fn claim_build_for_worker(
        &self,
        worker_id: &str,
    ) -> Result<Option<BuildRecord>, ApiError> {
        self.service.claim_build_for_worker(worker_id).await
    }

    /// Completes one claimed build for one worker identity.
    pub async fn complete_build_for_worker(
        &self,
        worker_id: &str,
        build_id: Uuid,
        status: WorkerBuildStatus,
        log_line: Option<String>,
    ) -> Result<BuildRecord, ApiError> {
        self.service
            .complete_build_for_worker(worker_id, build_id, status, log_line)
            .await
    }

    /// Runs one SCM polling tick and returns enqueue summary.
    pub async fn run_scm_polling_tick(&self) -> Result<ScmPollingTickResponse, ApiError> {
        self.service.run_scm_polling_tick().await
    }

    /// Ingests one normalized SCM webhook request.
    pub async fn ingest_scm_webhook(&self, request: &ScmWebhookRequest) -> Result<(), ApiError> {
        self.service.ingest_scm_webhook(request).await
    }

    /// Ingests one webhook request and records acceptance/rejection diagnostics in one place.
    pub async fn ingest_scm_webhook_observed(
        &self,
        request: &ScmWebhookRequest,
    ) -> Result<(), ScmWebhookIngestFailure> {
        let provider = request
            .header_value("x-scm-provider")
            .map(ToString::to_string);
        let repository_url = request
            .header_value("x-scm-repository")
            .map(ToString::to_string);

        self.service.record_scm_webhook_received();

        match self.service.ingest_scm_webhook(request).await {
            Ok(()) => {
                self.service.record_scm_webhook_accepted();
                Ok(())
            }
            Err(api_error) => {
                let failure = ScmWebhookIngestFailure::from_api_error(api_error);
                self.service.record_scm_webhook_rejected();
                self.service.record_scm_webhook_rejection(
                    failure.reason_code,
                    provider.as_deref(),
                    repository_url.as_deref(),
                );
                Err(failure)
            }
        }
    }

    /// Records that one SCM webhook request was received.
    pub fn record_scm_webhook_received(&self) {
        self.service.record_scm_webhook_received();
    }

    /// Records that one SCM webhook request was accepted.
    pub fn record_scm_webhook_accepted(&self) {
        self.service.record_scm_webhook_accepted();
    }

    /// Records that one SCM webhook request was rejected.
    pub fn record_scm_webhook_rejected(&self) {
        self.service.record_scm_webhook_rejected();
    }

    /// Stores one SCM webhook rejection diagnostic entry.
    pub fn record_scm_webhook_rejection(
        &self,
        reason_code: &str,
        provider: Option<&str>,
        repository_url: Option<&str>,
    ) {
        self.service
            .record_scm_webhook_rejection(reason_code, provider, repository_url);
    }

    /// Upserts repository-level webhook security configuration.
    pub async fn upsert_webhook_security_config(
        &self,
        mut config: WebhookSecurityConfig,
    ) -> Result<(), ApiError> {
        config.updated_at = Utc::now();
        self.service
            .storage
            .upsert_webhook_security_config(config)
            .await
            .map_err(|_| ApiError::Internal)
    }

    /// Upserts SCM polling configuration entry.
    pub async fn upsert_scm_polling_config(
        &self,
        mut config: ScmPollingConfig,
    ) -> Result<(), ApiError> {
        config.updated_at = Utc::now();
        self.service
            .storage
            .upsert_scm_polling_config(config)
            .await
            .map_err(|_| ApiError::Internal)
    }

    /// Starts background SCM polling loop using the configured check interval.
    pub fn start_scm_polling_loop(&self, check_interval: Duration) {
        let service = self.service.clone();
        tokio::spawn(async move {
            loop {
                let _ = service.run_scm_polling_tick().await;
                tokio::time::sleep(check_interval).await;
            }
        });
    }

    /// Lists jobs sorted by creation time.
    pub async fn list_jobs(&self) -> Result<Vec<JobDefinition>, ApiError> {
        self.service.list_jobs().await
    }

    /// Lists builds sorted by queue time.
    pub async fn list_builds(&self) -> Result<Vec<BuildRecord>, ApiError> {
        self.service.list_builds().await
    }

    /// Lists known workers and their current load.
    pub fn list_workers(&self) -> Result<Vec<WorkerInfo>, ApiError> {
        self.service.list_workers()
    }

    /// Lists builds currently in dead-letter state.
    pub async fn list_dead_letter_builds(&self) -> Result<Vec<BuildRecord>, ApiError> {
        self.service.list_dead_letter_builds().await
    }

    /// Runs readiness checks against core dependencies.
    pub async fn is_ready(&self) -> Result<(), ApiError> {
        self.service.is_ready().await
    }

    /// Returns current runtime metrics snapshot.
    pub fn metrics_snapshot(&self) -> RuntimeMetricsResponse {
        self.service.metrics_snapshot()
    }

    /// Lists recent SCM webhook rejections for diagnostics.
    pub fn list_scm_webhook_rejections(
        &self,
        provider: Option<&str>,
        repository_url: Option<&str>,
        limit: usize,
    ) -> Vec<ScmWebhookRejectionEntry> {
        self.service
            .list_scm_webhook_rejections(provider, repository_url, limit)
    }
}
