use async_graphql::{EmptySubscription, Schema};
use axum::{Extension, Router, http::HeaderMap, routing::{get, post}};
use chrono::{Duration as ChronoDuration, Utc};
use std::time::Duration;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use tardigrade_core::{
    BuildRecord, JobDefinition, PipelineDefinition, ScmProvider,
};
use tardigrade_executor::WorkerExecutor;
use tardigrade_scheduler::Scheduler;
use tardigrade_storage::Storage;
use tokio::sync::broadcast;
use uuid::Uuid;

mod events;
mod graphql;
mod handlers;
mod http_models;
mod service;
mod state;
mod settings;

pub use events::LiveEvent;
pub use http_models::{
    ApiErrorResponse, CancelBuildResponse, ClaimBuildResponse, CompleteBuildRequest,
    CompleteBuildResponse, CreateJobRequest, CreateJobResponse, DeadLetterBuildsResponse,
    HealthResponse, ListBuildsResponse, ListJobsResponse, ListWorkersResponse, LiveResponse,
    ReadyResponse, RunJobResponse, RuntimeMetricsResponse, ScmPollingTickResponse,
    ScmWebhookAcceptedResponse, UpsertScmPollingConfigRequest,
    UpsertWebhookSecurityConfigRequest, WorkerBuildStatus, WorkerInfo,
};
pub use state::ApiState;
pub use settings::ServiceSettings;
pub(crate) use graphql::CiGraphQLSchema;
use graphql::{MutationRoot, QueryRoot};
use handlers::{
    cancel_build, create_job, dead_letter_builds, events, graphql_handler, graphql_playground,
    health, ingest_scm_webhook, list_builds, list_jobs, list_workers, live, metrics, ready,
    run_job, run_scm_polling_tick, upsert_scm_polling_config, worker_claim_build,
    worker_complete_build,
};
pub(crate) use service::ApiError;
use service::{CiService, RuntimeMetrics, ScmTriggerEvent, map_pipeline_error};
use service::{
    build_webhook_dedup_key, header_value, parse_scm_provider_header, parse_scm_trigger_event,
    validate_ip_allowlist, validate_replay_window, verify_signature,
};

impl CiService {
    /// Creates orchestrator service from persistence, queue backend, and runtime settings.
    fn new(
        storage: Arc<dyn Storage + Send + Sync>,
        scheduler: Arc<dyn Scheduler + Send + Sync>,
        settings: ServiceSettings,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(512);

        Self {
            storage,
            scheduler,
            worker_registry: Arc::new(Mutex::new(HashMap::new())),
            worker_lease_timeout: Duration::from_secs(settings.worker_lease_timeout_secs),
            max_retries: settings.max_retries,
            retry_backoff_ms: settings.retry_backoff_ms,
            retry_state: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(RuntimeMetrics::default())),
            dead_letter_builds: Arc::new(Mutex::new(HashSet::new())),
            seen_webhook_events: Arc::new(Mutex::new(HashMap::new())),
            webhook_dedup_ttl: Duration::from_secs(settings.webhook_dedup_ttl_secs),
            event_tx,
        }
    }

    /// Subscribes to the in-process broadcast bus used by SSE clients.
    fn subscribe_events(&self) -> broadcast::Receiver<LiveEvent> {
        // Each subscriber receives live events from this point forward.
        self.event_tx.subscribe()
    }

    /// Emits one operational event to all connected live subscribers.
    fn emit_event(
        &self,
        kind: &str,
        severity: &str,
        message: impl Into<String>,
        job_id: Option<Uuid>,
        build_id: Option<Uuid>,
        worker_id: Option<&str>,
    ) {
        let _ = self.event_tx.send(LiveEvent {
            kind: kind.to_string(),
            severity: severity.to_string(),
            message: message.into(),
            job_id,
            build_id,
            worker_id: worker_id.map(ToString::to_string),
            at: Utc::now(),
        });
    }

    /// Reclaims stale worker leases and requeues corresponding builds.
    async fn reclaim_stale_builds(&self) -> Result<(), ApiError> {
        // Reclaim prevents stuck running builds after worker crashes/network partitions.
        let reclaimed = self
            .scheduler
            .reclaim_stale(self.worker_lease_timeout)
            .map_err(|_| ApiError::Internal)?;

        if !reclaimed.is_empty()
            && let Ok(mut metrics) = self.metrics.lock()
        {
            metrics.reclaimed_total += reclaimed.len() as u64;
        }

        for build_id in reclaimed {
            let Some(mut build) = self
                .storage
                .get_build(build_id)
                .await
                .map_err(|_| ApiError::Internal)?
            else {
                continue;
            };

            if build.requeue_from_running() {
                build.append_log("Requeued after stale worker lease timeout");
                self.storage
                    .save_build(build)
                    .await
                    .map_err(|_| ApiError::Internal)?;
                self.emit_event(
                    "build_reclaimed",
                    "warn",
                    "Build requeued after stale worker lease timeout",
                    None,
                    Some(build_id),
                    None,
                );
            }
        }

        Ok(())
    }

    /// Returns a consistent snapshot of runtime counters.
    fn metrics_snapshot(&self) -> RuntimeMetricsResponse {
        let metrics = self.metrics.lock().expect("metrics poisoned");
        RuntimeMetricsResponse {
            reclaimed_total: metrics.reclaimed_total,
            retry_requeued_total: metrics.retry_requeued_total,
            ownership_conflicts_total: metrics.ownership_conflicts_total,
            dead_letter_total: metrics.dead_letter_total,
            scm_webhook_received_total: metrics.scm_webhook_received_total,
            scm_webhook_accepted_total: metrics.scm_webhook_accepted_total,
            scm_webhook_rejected_total: metrics.scm_webhook_rejected_total,
            scm_webhook_duplicate_total: metrics.scm_webhook_duplicate_total,
            scm_trigger_enqueued_builds_total: metrics.scm_trigger_enqueued_builds_total,
            scm_polling_ticks_total: metrics.scm_polling_ticks_total,
            scm_polling_repositories_total: metrics.scm_polling_repositories_total,
            scm_polling_enqueued_builds_total: metrics.scm_polling_enqueued_builds_total,
        }
    }

    /// Records one received SCM webhook request before validation outcome is known.
    fn record_scm_webhook_received(&self) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.scm_webhook_received_total += 1;
        }
    }

    /// Records one accepted SCM webhook request (`202`) after ingestion succeeded.
    fn record_scm_webhook_accepted(&self) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.scm_webhook_accepted_total += 1;
        }
    }

    /// Records one rejected SCM webhook request after validation or processing error.
    fn record_scm_webhook_rejected(&self) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.scm_webhook_rejected_total += 1;
        }
    }

    /// Updates worker heartbeat timestamp for dashboard visibility.
    fn touch_worker(&self, worker_id: &str) {
        if let Ok(mut registry) = self.worker_registry.lock() {
            registry.insert(worker_id.to_string(), Utc::now());
        }
    }

    /// Validates and persists a new job definition.
    async fn create_job(&self, payload: CreateJobRequest) -> Result<JobDefinition, ApiError> {
        if payload.name.trim().is_empty()
            || payload.repository_url.trim().is_empty()
            || payload.pipeline_path.trim().is_empty()
        {
            return Err(ApiError::BadRequest);
        }

        if let Some(pipeline_yaml) = payload.pipeline_yaml.as_ref() {
            if pipeline_yaml.trim().is_empty() {
                return Err(ApiError::BadRequest);
            }

            PipelineDefinition::from_yaml_str(pipeline_yaml).map_err(map_pipeline_error)?;
        }

        let job = JobDefinition::new(payload.name, payload.repository_url, payload.pipeline_path);
        self.storage
            .save_job(job.clone())
            .await
            .map_err(|_| ApiError::Internal)?;

        self.emit_event(
            "job_created",
            "info",
            format!("Job {} created", job.name),
            Some(job.id),
            None,
            None,
        );

        Ok(job)
    }

    /// Validates and accepts one SCM webhook after signature, replay, and allowlist checks.
    async fn ingest_scm_webhook(
        &self,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<(), ApiError> {
        let provider = parse_scm_provider_header(headers)?;
        let repository_url = header_value(headers, "x-scm-repository")?;
        let config = self
            .storage
            .get_webhook_security_config(&repository_url, provider)
            .await
            .map_err(|_| ApiError::Internal)?
            .ok_or(ApiError::Forbidden)?;

        validate_replay_window(headers, Duration::from_secs(5 * 60))?;
        validate_ip_allowlist(headers, &config.allowed_ips)?;
        verify_signature(provider, headers, body, &config.secret)?;
        let event = parse_scm_trigger_event(provider, headers, body)?;

        if let Some(event) = event {
            if let Some(dedup_key) = build_webhook_dedup_key(
                provider,
                &repository_url,
                headers,
                body,
                event,
            ) {
                if self.is_duplicate_webhook_event(&dedup_key) {
                    if let Ok(mut metrics) = self.metrics.lock() {
                        metrics.scm_webhook_duplicate_total += 1;
                    }
                    self.emit_event(
                        "scm_webhook_duplicate_ignored",
                        "info",
                        format!(
                            "Duplicate webhook ignored for repository {}",
                            repository_url
                        ),
                        None,
                        None,
                        None,
                    );
                    return Ok(());
                }
            }

            self.enqueue_repository_jobs_for_event(&repository_url, event)
                .await?;
        }

        self.emit_event(
            "scm_webhook_ingested",
            "info",
            format!("Webhook accepted for repository {}", repository_url),
            None,
            None,
            None,
        );

        Ok(())
    }

    /// Returns true when a webhook dedup key is still within TTL and should be ignored.
    fn is_duplicate_webhook_event(&self, dedup_key: &str) -> bool {
        let now = Utc::now();
        let ttl = ChronoDuration::from_std(self.webhook_dedup_ttl)
            .unwrap_or_else(|_| ChronoDuration::seconds(0));

        let mut seen = self
            .seen_webhook_events
            .lock()
            .expect("webhook dedup state poisoned");
        seen.retain(|_, seen_at| now.signed_duration_since(*seen_at) <= ttl);

        if seen.contains_key(dedup_key) {
            return true;
        }

        seen.insert(dedup_key.to_string(), now);
        false
    }

    /// Enqueues builds for all jobs bound to one repository when a trigger event is accepted.
    async fn enqueue_repository_jobs_for_event(
        &self,
        repository_url: &str,
        event: ScmTriggerEvent,
    ) -> Result<(), ApiError> {
        let jobs = self
            .storage
            .list_jobs()
            .await
            .map_err(|_| ApiError::Internal)?;

        let mut triggered = 0usize;
        for job in jobs
            .into_iter()
            .filter(|job| job.repository_url == repository_url)
        {
            let _ = self.run_job(job.id).await?;
            triggered += 1;
        }

        self.emit_event(
            "scm_trigger_processed",
            "info",
            format!(
                "SCM event {:?} processed for repository {} ({} job(s) enqueued)",
                event, repository_url, triggered
            ),
            None,
            None,
            None,
        );

        if triggered > 0
            && let Ok(mut metrics) = self.metrics.lock()
        {
            metrics.scm_trigger_enqueued_builds_total += triggered as u64;
        }

        Ok(())
    }

    /// Runs one SCM polling tick and enqueues builds for due repository configs.
    async fn run_scm_polling_tick(&self) -> Result<ScmPollingTickResponse, ApiError> {
        let now = Utc::now();
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.scm_polling_ticks_total += 1;
        }
        let configs = self
            .storage
            .list_scm_polling_configs()
            .await
            .map_err(|_| ApiError::Internal)?;

        let mut polled_repositories = 0usize;
        let mut enqueued_builds = 0usize;

        for mut config in configs {
            if !config.enabled {
                continue;
            }

            if let Some(last) = config.last_polled_at {
                let elapsed = now - last;
                if elapsed.num_seconds() < i64::try_from(config.interval_secs).unwrap_or(i64::MAX)
                {
                    continue;
                }
            }

            polled_repositories += 1;
            let jobs = self
                .storage
                .list_jobs()
                .await
                .map_err(|_| ApiError::Internal)?;

            for job in jobs
                .into_iter()
                .filter(|job| job.repository_url == config.repository_url)
            {
                let _ = self.run_job(job.id).await?;
                enqueued_builds += 1;
            }

            config.last_polled_at = Some(now);
            config.updated_at = now;
            self.storage
                .upsert_scm_polling_config(config)
                .await
                .map_err(|_| ApiError::Internal)?;
        }

        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.scm_polling_repositories_total += polled_repositories as u64;
            metrics.scm_polling_enqueued_builds_total += enqueued_builds as u64;
        }

        Ok(ScmPollingTickResponse {
            polled_repositories,
            enqueued_builds,
        })
    }

    /// Lists jobs sorted chronologically by creation time.
    async fn list_jobs(&self) -> Result<Vec<JobDefinition>, ApiError> {
        let mut jobs = self
            .storage
            .list_jobs()
            .await
            .map_err(|_| ApiError::Internal)?;
        jobs.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(jobs)
    }

    /// Creates and enqueues a build for a known job.
    async fn run_job(&self, job_id: Uuid) -> Result<BuildRecord, ApiError> {
        let Some(_) = self
            .storage
            .get_job(job_id)
            .await
            .map_err(|_| ApiError::Internal)?
        else {
            return Err(ApiError::NotFound);
        };

        let mut build = BuildRecord::queued(job_id);
        build.append_log("Queued by API");
        self.storage
            .save_build(build.clone())
            .await
            .map_err(|_| ApiError::Internal)?;
        self.scheduler
            .enqueue(build.id)
            .map_err(|_| ApiError::Internal)?;

        self.emit_event(
            "build_queued",
            "info",
            format!("Build {} queued", build.id),
            Some(job_id),
            Some(build.id),
            None,
        );

        Ok(build)
    }

    /// Embedded-worker loop: claim next build, execute it, and ack queue ownership.
    async fn process_next_build(&self) -> Result<(), ApiError> {
        let Some(build) = self.claim_build_for_worker("embedded-worker").await? else {
            return Ok(());
        };

        let build_id = build.id;
        let executed = match WorkerExecutor::run(build).await {
            Ok(done) => done,
            Err(_) => {
                self.scheduler
                    .requeue(build_id)
                    .map_err(|_| ApiError::Internal)?;
                return Err(ApiError::Internal);
            }
        };

        self.storage
            .save_build(executed)
            .await
            .map_err(|_| ApiError::Internal)?;
        self.scheduler
            .ack(build_id)
            .map_err(|_| ApiError::Internal)?;
        Ok(())
    }

    /// Claims one build for worker and transitions state to running when possible.
    async fn claim_build_for_worker(
        &self,
        worker_id: &str,
    ) -> Result<Option<BuildRecord>, ApiError> {
        self.touch_worker(worker_id);
        // Claim path always tries to reclaim stale leases before taking new work.
        self.reclaim_stale_builds().await?;

        let Some(build_id) = self.scheduler.claim_next(worker_id) else {
            return Ok(None);
        };

        let Some(mut build) = self
            .storage
            .get_build(build_id)
            .await
            .map_err(|_| ApiError::Internal)?
        else {
            self.scheduler
                .ack(build_id)
                .map_err(|_| ApiError::Internal)?;
            return Ok(None);
        };

        if !build.mark_running() {
            // Another actor may have completed or canceled it.
            self.scheduler
                .ack(build_id)
                .map_err(|_| ApiError::Internal)?;
            return Ok(None);
        }

        build.append_log(format!("Claimed by worker {worker_id}"));
        self.storage
            .save_build(build.clone())
            .await
            .map_err(|_| ApiError::Internal)?;
        self.emit_event(
            "build_claimed",
            "info",
            format!("Build {} claimed", build.id),
            Some(build.job_id),
            Some(build.id),
            Some(worker_id),
        );
        Ok(Some(build))
    }

    /// Finalizes one claimed build with ownership checks, retry policy, and dead-letter handling.
    async fn complete_build_for_worker(
        &self,
        worker_id: &str,
        build_id: Uuid,
        status: WorkerBuildStatus,
        log_line: Option<String>,
    ) -> Result<BuildRecord, ApiError> {
        self.touch_worker(worker_id);

        let owner = self
            .scheduler
            .in_flight_owner(build_id)
            .map_err(|_| ApiError::Internal)?;
        // Ownership is enforced so one worker cannot complete another worker's build.
        if owner.as_deref() != Some(worker_id) {
            if let Ok(mut metrics) = self.metrics.lock() {
                metrics.ownership_conflicts_total += 1;
            }
            self.emit_event(
                "ownership_conflict",
                "warn",
                format!("Ownership conflict on build {}", build_id),
                None,
                Some(build_id),
                Some(worker_id),
            );
            return Err(ApiError::Conflict);
        }

        let Some(mut build) = self
            .storage
            .get_build(build_id)
            .await
            .map_err(|_| ApiError::Internal)?
        else {
            return Err(ApiError::NotFound);
        };

        if let Some(line) = log_line {
            build.append_log(line);
        }

        match status {
            WorkerBuildStatus::Success => {
                if let Ok(mut retry_state) = self.retry_state.lock() {
                    retry_state.remove(&build_id);
                }
                if let Ok(mut dead_letter) = self.dead_letter_builds.lock() {
                    dead_letter.remove(&build_id);
                }
                if build.mark_success() {
                    build.append_log(format!("Completed successfully by worker {worker_id}"));
                    self.emit_event(
                        "build_succeeded",
                        "ok",
                        format!("Build {} completed successfully", build.id),
                        Some(build.job_id),
                        Some(build.id),
                        Some(worker_id),
                    );
                }
            }
            WorkerBuildStatus::Failed => {
                let attempt = {
                    let mut retry_state = self.retry_state.lock().expect("retry state poisoned");
                    let entry = retry_state.entry(build_id).or_insert(0);
                    *entry += 1;
                    *entry
                };

                if attempt <= self.max_retries && build.requeue_from_running() {
                    // Exponential backoff caps growth to avoid extreme waits on long retry chains.
                    let exp = (attempt.saturating_sub(1)).min(8);
                    let factor = 1u64 << exp;
                    let delay_ms = self.retry_backoff_ms.saturating_mul(factor);
                    let delay = Duration::from_millis(delay_ms);

                    build.append_log(format!(
                        "Worker {worker_id} reported failure, retry {attempt}/{max} scheduled in {delay_ms}ms",
                        max = self.max_retries
                    ));

                    self.storage
                        .save_build(build.clone())
                        .await
                        .map_err(|_| ApiError::Internal)?;
                    self.scheduler
                        .ack(build_id)
                        .map_err(|_| ApiError::Internal)?;

                    if let Ok(mut metrics) = self.metrics.lock() {
                        metrics.retry_requeued_total += 1;
                    }

                    self.emit_event(
                        "build_requeued",
                        "warn",
                        format!(
                            "Build {} failed on worker {}. Retry {}/{} scheduled",
                            build.id, worker_id, attempt, self.max_retries
                        ),
                        Some(build.job_id),
                        Some(build.id),
                        Some(worker_id),
                    );

                    let scheduler = self.scheduler.clone();
                    tokio::spawn(async move {
                        // Requeue happens asynchronously so API response remains fast.
                        tokio::time::sleep(delay).await;
                        let _ = scheduler.requeue(build_id);
                    });

                    return Ok(build);
                }

                if let Ok(mut retry_state) = self.retry_state.lock() {
                    retry_state.remove(&build_id);
                }

                if build.mark_failed() {
                    // Build is moved to dead-letter after final retry is exhausted.
                    build.append_log(format!(
                        "Failed by worker {worker_id} after retries (moved to dead-letter)"
                    ));
                    if let Ok(mut dead_letter) = self.dead_letter_builds.lock() {
                        dead_letter.insert(build_id);
                    }
                    if let Ok(mut metrics) = self.metrics.lock() {
                        metrics.dead_letter_total += 1;
                    }
                    self.emit_event(
                        "build_dead_lettered",
                        "error",
                        format!("Build {} moved to dead-letter", build.id),
                        Some(build.job_id),
                        Some(build.id),
                        Some(worker_id),
                    );
                }
            }
        }

        self.storage
            .save_build(build.clone())
            .await
            .map_err(|_| ApiError::Internal)?;
        self.scheduler
            .ack(build_id)
            .map_err(|_| ApiError::Internal)?;
        Ok(build)
    }

    /// Materializes dead-letter builds for operator-focused API/dashboard views.
    async fn list_dead_letter_builds(&self) -> Result<Vec<BuildRecord>, ApiError> {
        let dead_letter_ids = self
            .dead_letter_builds
            .lock()
            .map_err(|_| ApiError::Internal)?
            .iter()
            .copied()
            .collect::<Vec<_>>();

        let mut builds = Vec::new();
        for build_id in dead_letter_ids {
            if let Some(build) = self
                .storage
                .get_build(build_id)
                .await
                .map_err(|_| ApiError::Internal)?
            {
                builds.push(build);
            }
        }

        builds.sort_by(|a, b| b.queued_at.cmp(&a.queued_at));
        Ok(builds)
    }

    /// Lists known workers enriched with active build counts.
    fn list_workers(&self) -> Result<Vec<WorkerInfo>, ApiError> {
        let loads = self.scheduler.worker_loads();
        let registry = self
            .worker_registry
            .lock()
            .map_err(|_| ApiError::Internal)?;

        let mut workers = registry
            .iter()
            .map(|(id, last_seen_at)| {
                let active_builds = *loads.get(id).unwrap_or(&0);
                let status = if active_builds > 0 {
                    "busy".to_string()
                } else {
                    "idle".to_string()
                };

                WorkerInfo {
                    id: id.clone(),
                    active_builds,
                    status,
                    last_seen_at: last_seen_at.to_owned(),
                }
            })
            .collect::<Vec<_>>();

        workers.sort_by(|a, b| b.last_seen_at.cmp(&a.last_seen_at));
        Ok(workers)
    }

    /// Readiness check ensuring core dependencies are reachable.
    async fn is_ready(&self) -> Result<(), ApiError> {
        // Readiness checks that core dependencies are reachable.
        self.storage
            .list_jobs()
            .await
            .map_err(|_| ApiError::Internal)?;
        let _ = self.scheduler.worker_loads();
        Ok(())
    }

    /// Lists builds sorted by queue time (newest first).
    async fn list_builds(&self) -> Result<Vec<BuildRecord>, ApiError> {
        let mut builds = self
            .storage
            .list_builds()
            .await
            .map_err(|_| ApiError::Internal)?;
        builds.sort_by(|a, b| b.queued_at.cmp(&a.queued_at));
        Ok(builds)
    }

    /// Cancels one build and persists resulting state.
    async fn cancel_build(&self, build_id: Uuid) -> Result<BuildRecord, ApiError> {
        let Some(mut build) = self
            .storage
            .get_build(build_id)
            .await
            .map_err(|_| ApiError::Internal)?
        else {
            return Err(ApiError::NotFound);
        };

        if build.cancel() {
            build.append_log("Canceled by API");
            self.emit_event(
                "build_canceled",
                "warn",
                format!("Build {} canceled", build.id),
                Some(build.job_id),
                Some(build.id),
                None,
            );
        }

        self.storage
            .save_build(build.clone())
            .await
            .map_err(|_| ApiError::Internal)?;
        Ok(build)
    }
}

/// Builds the full HTTP router for CI control-plane API.
pub fn build_router(state: ApiState) -> Router {
    let graphql_schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(state.clone())
        .finish();

    // Router keeps control-plane endpoints grouped by capability:
    // liveness/readiness, jobs/builds, workers, and operations telemetry.
    Router::new()
        .route("/health", get(health))
        .route("/live", get(live))
        .route("/ready", get(ready))
        .route("/graphql", get(graphql_playground).post(graphql_handler))
        .route("/events", get(events))
        .route("/metrics", get(metrics))
        .route("/dead-letter-builds", get(dead_letter_builds))
        .route("/jobs", post(create_job).get(list_jobs))
        .route("/builds", get(list_builds))
        .route("/workers", get(list_workers))
        .route("/webhooks/scm", post(ingest_scm_webhook))
        .route("/scm/polling/configs", post(upsert_scm_polling_config))
        .route("/scm/polling/tick", post(run_scm_polling_tick))
        .route("/jobs/{id}/run", post(run_job))
        .route("/builds/{id}/cancel", post(cancel_build))
        .route("/workers/{worker_id}/claim", post(worker_claim_build))
        .route(
            "/workers/{worker_id}/builds/{id}/complete",
            post(worker_complete_build),
        )
        .layer(Extension(graphql_schema))
        .with_state(state)
}

