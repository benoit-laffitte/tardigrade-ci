use async_graphql::{
    Context, EmptySubscription, Enum, Error as GraphQLError, ID, InputObject, Object, Schema,
    SimpleObject,
    http::{GraphQLPlaygroundConfig, playground_source},
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, sse::{Event, KeepAlive, Sse}},
    routing::{get, post},
};
use tardigrade_executor::WorkerExecutor;
use tardigrade_core::{BuildRecord, JobDefinition, JobStatus};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use tardigrade_scheduler::{InMemoryScheduler, Scheduler};
use tardigrade_storage::{InMemoryStorage, Storage};
use uuid::Uuid;

#[derive(Clone)]
pub struct ApiState {
    pub service_name: String,
    /// Service owns all domain orchestration (storage, scheduler, metrics, events).
    service: Arc<CiService>,
    run_embedded_worker: bool,
}

/// Runtime tuning knobs for reliability behavior (leases/retries/backoff).
#[derive(Clone, Copy)]
pub struct ServiceSettings {
    pub worker_lease_timeout_secs: u64,
    pub max_retries: u32,
    pub retry_backoff_ms: u64,
}

impl ServiceSettings {
    /// Loads reliability settings from environment variables with safe defaults.
    pub fn from_env() -> Self {
        // Env-based defaults keep local dev easy while allowing production tuning.
        let worker_lease_timeout_secs = std::env::var("TARDIGRADE_WORKER_LEASE_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);
        let max_retries = std::env::var("TARDIGRADE_BUILD_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(2);
        let retry_backoff_ms = std::env::var("TARDIGRADE_BUILD_RETRY_BACKOFF_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1000);

        Self {
            worker_lease_timeout_secs,
            max_retries,
            retry_backoff_ms,
        }
    }
}

/// Response body for service health endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: String,
}

/// Response body for process liveness endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct LiveResponse {
    pub status: &'static str,
}

/// Response body for readiness endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadyResponse {
    pub status: &'static str,
}

/// Request payload used to create a new job.
#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub name: String,
    pub repository_url: String,
    pub pipeline_path: String,
}

/// Response payload containing the created job.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateJobResponse {
    pub job: JobDefinition,
}

/// Response payload listing known jobs.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListJobsResponse {
    pub jobs: Vec<JobDefinition>,
}

/// Response payload containing enqueued build record.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunJobResponse {
    pub build: BuildRecord,
}

/// Response payload containing canceled build state.
#[derive(Debug, Serialize, Deserialize)]
pub struct CancelBuildResponse {
    pub build: BuildRecord,
}

/// Response payload listing builds.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListBuildsResponse {
    pub builds: Vec<BuildRecord>,
}

/// Worker telemetry model shown by dashboard.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub id: String,
    pub active_builds: usize,
    pub status: String,
    pub last_seen_at: DateTime<Utc>,
}

/// Response payload listing workers and their current loads.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListWorkersResponse {
    pub workers: Vec<WorkerInfo>,
}

/// Response payload for worker claim call.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClaimBuildResponse {
    pub build: Option<BuildRecord>,
}

/// Worker-reported terminal result for one build execution.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerBuildStatus {
    Success,
    Failed,
}

/// Request payload for worker completion call.
#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteBuildRequest {
    pub status: WorkerBuildStatus,
    pub log_line: Option<String>,
}

/// Response payload containing updated build after completion.
#[derive(Debug, Serialize, Deserialize)]
pub struct CompleteBuildResponse {
    pub build: BuildRecord,
}

/// Runtime reliability counters exposed for operators.
#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeMetricsResponse {
    pub reclaimed_total: u64,
    pub retry_requeued_total: u64,
    pub ownership_conflicts_total: u64,
    pub dead_letter_total: u64,
}

/// Response payload listing builds moved to dead-letter set.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeadLetterBuildsResponse {
    pub builds: Vec<BuildRecord>,
}

/// Live event model emitted by the API and streamed to dashboard clients.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveEvent {
    pub kind: String,
    pub severity: String,
    pub message: String,
    pub job_id: Option<Uuid>,
    pub build_id: Option<Uuid>,
    pub worker_id: Option<String>,
    pub at: DateTime<Utc>,
}

/// GraphQL schema serving CI query and mutation operations.
type CiGraphQLSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

/// GraphQL query root exposing read-oriented CI operations.
pub struct QueryRoot;

/// GraphQL mutation root exposing write-oriented CI operations.
pub struct MutationRoot;

/// GraphQL projection for health endpoint response.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
struct GqlHealthResponse {
    status: String,
    service: String,
}

/// GraphQL projection for liveness endpoint response.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
struct GqlLiveResponse {
    status: String,
}

/// GraphQL projection for readiness endpoint response.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
struct GqlReadyResponse {
    status: String,
}

/// GraphQL enum mirroring runtime build lifecycle statuses.
#[derive(Clone, Copy, Eq, PartialEq, Enum)]
enum GqlJobStatus {
    Pending,
    Running,
    Success,
    Failed,
    Canceled,
}

/// GraphQL projection for persisted job definitions.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
struct GqlJobDefinition {
    id: ID,
    name: String,
    repository_url: String,
    pipeline_path: String,
    created_at: String,
}

/// GraphQL projection for persisted build records.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
struct GqlBuildRecord {
    id: ID,
    job_id: ID,
    status: GqlJobStatus,
    queued_at: String,
    started_at: Option<String>,
    finished_at: Option<String>,
    logs: Vec<String>,
}

/// GraphQL projection for worker telemetry card.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
struct GqlWorkerInfo {
    id: String,
    active_builds: usize,
    status: String,
    last_seen_at: String,
}

/// GraphQL projection for runtime reliability counters.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
struct GqlRuntimeMetrics {
    reclaimed_total: u64,
    retry_requeued_total: u64,
    ownership_conflicts_total: u64,
    dead_letter_total: u64,
}

/// GraphQL projection grouping dashboard panels into a single payload.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
struct GqlDashboardSnapshot {
    jobs: Vec<GqlJobDefinition>,
    builds: Vec<GqlBuildRecord>,
    workers: Vec<GqlWorkerInfo>,
    metrics: GqlRuntimeMetrics,
    dead_letter_builds: Vec<GqlBuildRecord>,
}

/// Worker-reported terminal status accepted by GraphQL completion mutation.
#[derive(Clone, Copy, Eq, PartialEq, Enum)]
enum GqlWorkerBuildStatus {
    Success,
    Failed,
}

/// GraphQL input used by create_job mutation.
#[derive(InputObject)]
#[graphql(rename_fields = "snake_case")]
struct GqlCreateJobInput {
    name: String,
    repository_url: String,
    pipeline_path: String,
}

impl From<JobStatus> for GqlJobStatus {
    fn from(value: JobStatus) -> Self {
        match value {
            JobStatus::Pending => Self::Pending,
            JobStatus::Running => Self::Running,
            JobStatus::Success => Self::Success,
            JobStatus::Failed => Self::Failed,
            JobStatus::Canceled => Self::Canceled,
        }
    }
}

impl From<JobDefinition> for GqlJobDefinition {
    fn from(value: JobDefinition) -> Self {
        Self {
            id: ID(value.id.to_string()),
            name: value.name,
            repository_url: value.repository_url,
            pipeline_path: value.pipeline_path,
            created_at: value.created_at.to_rfc3339(),
        }
    }
}

impl From<BuildRecord> for GqlBuildRecord {
    fn from(value: BuildRecord) -> Self {
        Self {
            id: ID(value.id.to_string()),
            job_id: ID(value.job_id.to_string()),
            status: value.status.into(),
            queued_at: value.queued_at.to_rfc3339(),
            started_at: value.started_at.map(|dt| dt.to_rfc3339()),
            finished_at: value.finished_at.map(|dt| dt.to_rfc3339()),
            logs: value.logs,
        }
    }
}

impl From<WorkerInfo> for GqlWorkerInfo {
    fn from(value: WorkerInfo) -> Self {
        Self {
            id: value.id,
            active_builds: value.active_builds,
            status: value.status,
            last_seen_at: value.last_seen_at.to_rfc3339(),
        }
    }
}

impl From<RuntimeMetricsResponse> for GqlRuntimeMetrics {
    fn from(value: RuntimeMetricsResponse) -> Self {
        Self {
            reclaimed_total: value.reclaimed_total,
            retry_requeued_total: value.retry_requeued_total,
            ownership_conflicts_total: value.ownership_conflicts_total,
            dead_letter_total: value.dead_letter_total,
        }
    }
}

fn parse_id_as_uuid(id: &ID) -> Result<Uuid, GraphQLError> {
    Uuid::parse_str(id.as_str()).map_err(|_| GraphQLError::new("invalid UUID id"))
}

fn gql_err_from_api(err: ApiError) -> GraphQLError {
    GraphQLError::new(format!("request failed with status {}", err.status_code().as_u16()))
}

#[Object(rename_fields = "snake_case")]
impl QueryRoot {
    /// Returns service identity and health status.
    async fn health(&self, ctx: &Context<'_>) -> GqlHealthResponse {
        let state = ctx.data_unchecked::<ApiState>();
        GqlHealthResponse {
            status: "ok".to_string(),
            service: state.service_name.clone(),
        }
    }

    /// Returns process liveness status.
    async fn live(&self) -> GqlLiveResponse {
        GqlLiveResponse {
            status: "alive".to_string(),
        }
    }

    /// Returns readiness status after dependency checks.
    async fn ready(&self, ctx: &Context<'_>) -> Result<GqlReadyResponse, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        state
            .service
            .is_ready()
            .await
            .map_err(gql_err_from_api)?;
        Ok(GqlReadyResponse {
            status: "ready".to_string(),
        })
    }

    /// Returns jobs list sorted by creation time.
    async fn jobs(&self, ctx: &Context<'_>) -> Result<Vec<GqlJobDefinition>, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let jobs = state.service.list_jobs().await.map_err(gql_err_from_api)?;
        Ok(jobs.into_iter().map(Into::into).collect())
    }

    /// Returns builds list sorted by queue time.
    async fn builds(&self, ctx: &Context<'_>) -> Result<Vec<GqlBuildRecord>, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let builds = state.service.list_builds().await.map_err(gql_err_from_api)?;
        Ok(builds.into_iter().map(Into::into).collect())
    }

    /// Returns worker telemetry and current load.
    async fn workers(&self, ctx: &Context<'_>) -> Result<Vec<GqlWorkerInfo>, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let workers = state.service.list_workers().map_err(gql_err_from_api)?;
        Ok(workers.into_iter().map(Into::into).collect())
    }

    /// Returns runtime reliability counters.
    async fn metrics(&self, ctx: &Context<'_>) -> GqlRuntimeMetrics {
        let state = ctx.data_unchecked::<ApiState>();
        state.service.metrics_snapshot().into()
    }

    /// Returns builds currently moved to dead-letter set.
    async fn dead_letter_builds(&self, ctx: &Context<'_>) -> Result<Vec<GqlBuildRecord>, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let builds = state
            .service
            .list_dead_letter_builds()
            .await
            .map_err(gql_err_from_api)?;
        Ok(builds.into_iter().map(Into::into).collect())
    }

    /// Returns full dashboard snapshot in a single request.
    async fn dashboard_snapshot(&self, ctx: &Context<'_>) -> Result<GqlDashboardSnapshot, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let jobs = state.service.list_jobs().await.map_err(gql_err_from_api)?;
        let builds = state.service.list_builds().await.map_err(gql_err_from_api)?;
        let workers = state.service.list_workers().map_err(gql_err_from_api)?;
        let dead_letter_builds = state
            .service
            .list_dead_letter_builds()
            .await
            .map_err(gql_err_from_api)?;

        Ok(GqlDashboardSnapshot {
            jobs: jobs.into_iter().map(Into::into).collect(),
            builds: builds.into_iter().map(Into::into).collect(),
            workers: workers.into_iter().map(Into::into).collect(),
            metrics: state.service.metrics_snapshot().into(),
            dead_letter_builds: dead_letter_builds.into_iter().map(Into::into).collect(),
        })
    }
}

#[Object(rename_fields = "snake_case")]
impl MutationRoot {
    /// Creates one job definition and persists it.
    async fn create_job(
        &self,
        ctx: &Context<'_>,
        input: GqlCreateJobInput,
    ) -> Result<GqlJobDefinition, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let job = state
            .service
            .create_job(CreateJobRequest {
                name: input.name,
                repository_url: input.repository_url,
                pipeline_path: input.pipeline_path,
            })
            .await
            .map_err(gql_err_from_api)?;
        Ok(job.into())
    }

    /// Enqueues one build for the specified job id.
    async fn run_job(&self, ctx: &Context<'_>, job_id: ID) -> Result<GqlBuildRecord, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let job_uuid = parse_id_as_uuid(&job_id)?;
        let build = state
            .service
            .run_job(job_uuid)
            .await
            .map_err(gql_err_from_api)?;

        if state.run_embedded_worker {
            let service = state.service.clone();
            tokio::spawn(async move {
                let _ = service.process_next_build().await;
            });
        }

        Ok(build.into())
    }

    /// Cancels one build by id.
    async fn cancel_build(&self, ctx: &Context<'_>, build_id: ID) -> Result<GqlBuildRecord, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let build_uuid = parse_id_as_uuid(&build_id)?;
        let build = state
            .service
            .cancel_build(build_uuid)
            .await
            .map_err(gql_err_from_api)?;
        Ok(build.into())
    }

    /// Claims one build for worker and marks it running.
    async fn worker_claim_build(
        &self,
        ctx: &Context<'_>,
        worker_id: String,
    ) -> Result<Option<GqlBuildRecord>, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let build = state
            .service
            .claim_build_for_worker(&worker_id)
            .await
            .map_err(gql_err_from_api)?;
        Ok(build.map(Into::into))
    }

    /// Completes one worker-owned build and applies retry/dead-letter policy.
    async fn worker_complete_build(
        &self,
        ctx: &Context<'_>,
        worker_id: String,
        build_id: ID,
        status: GqlWorkerBuildStatus,
        log_line: Option<String>,
    ) -> Result<GqlBuildRecord, GraphQLError> {
        let state = ctx.data_unchecked::<ApiState>();
        let build_uuid = parse_id_as_uuid(&build_id)?;
        let status = match status {
            GqlWorkerBuildStatus::Success => WorkerBuildStatus::Success,
            GqlWorkerBuildStatus::Failed => WorkerBuildStatus::Failed,
        };

        let build = state
            .service
            .complete_build_for_worker(&worker_id, build_uuid, status, log_line)
            .await
            .map_err(gql_err_from_api)?;
        Ok(build.into())
    }
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
}

#[derive(Clone)]
struct CiService {
    storage: Arc<dyn Storage + Send + Sync>,
    scheduler: Arc<dyn Scheduler + Send + Sync>,
    /// last_seen map allows the dashboard to expose active/idle workers.
    worker_registry: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    worker_lease_timeout: Duration,
    max_retries: u32,
    retry_backoff_ms: u64,
    /// retry_state tracks attempt count per build until terminal state.
    retry_state: Arc<Mutex<HashMap<Uuid, u32>>>,
    metrics: Arc<Mutex<RuntimeMetrics>>,
    /// dead_letter_builds provides a focused operational view over failed terminal retries.
    dead_letter_builds: Arc<Mutex<HashSet<Uuid>>>,
    /// Internal broadcast bus feeding the SSE /events endpoint.
    event_tx: broadcast::Sender<LiveEvent>,
}

/// Mutable runtime counters for reliability-oriented observability.
#[derive(Default)]
struct RuntimeMetrics {
    reclaimed_total: u64,
    retry_requeued_total: u64,
    ownership_conflicts_total: u64,
    dead_letter_total: u64,
}

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

        if !reclaimed.is_empty() {
            if let Ok(mut metrics) = self.metrics.lock() {
                metrics.reclaimed_total += reclaimed.len() as u64;
            }
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
        self.scheduler.ack(build_id).map_err(|_| ApiError::Internal)?;
        Ok(())
    }

    /// Claims one build for worker and transitions state to running when possible.
    async fn claim_build_for_worker(&self, worker_id: &str) -> Result<Option<BuildRecord>, ApiError> {
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
            self.scheduler.ack(build_id).map_err(|_| ApiError::Internal)?;
            return Ok(None);
        };

        if !build.mark_running() {
            // Another actor may have completed or canceled it.
            self.scheduler.ack(build_id).map_err(|_| ApiError::Internal)?;
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
                    self.scheduler.ack(build_id).map_err(|_| ApiError::Internal)?;

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
                    build.append_log(format!("Failed by worker {worker_id} after retries (moved to dead-letter)"));
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
        self.scheduler.ack(build_id).map_err(|_| ApiError::Internal)?;
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
        let registry = self.worker_registry.lock().map_err(|_| ApiError::Internal)?;

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

/// Internal service-layer error taxonomy converted to HTTP codes at the edge.
enum ApiError {
    BadRequest,
    NotFound,
    Conflict,
    Internal,
}

impl ApiError {
    /// Maps domain/service errors to stable HTTP status codes.
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
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

/// Serves GraphQL playground for interactive schema exploration.
async fn graphql_playground() -> Html<String> {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

/// Executes one GraphQL request against CI schema.
async fn graphql_handler(
    Extension(schema): Extension<CiGraphQLSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

/// Returns service identity and basic health signal.
async fn health(State(state): State<ApiState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: state.service_name,
    })
}

/// Returns process liveness probe response.
async fn live() -> Json<LiveResponse> {
    Json(LiveResponse { status: "alive" })
}

/// Returns readiness probe response after dependency checks.
async fn ready(State(state): State<ApiState>) -> Result<(StatusCode, Json<ReadyResponse>), StatusCode> {
    state.service.is_ready().await.map_err(|e| e.status_code())?;
    Ok((StatusCode::OK, Json(ReadyResponse { status: "ready" })))
}

/// Streams live operational events to dashboard clients using SSE.
async fn events(State(state): State<ApiState>) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    // BroadcastStream may drop lagging messages; dashboard treats this as best-effort live feed.
    let stream = BroadcastStream::new(state.service.subscribe_events()).filter_map(|msg| {
        match msg {
            Ok(event) => {
                let data = serde_json::to_string(&event).ok()?;
                Some(Ok(Event::default().data(data)))
            }
            Err(_) => None,
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("keep-alive"))
}

/// Returns current reliability metrics snapshot.
async fn metrics(State(state): State<ApiState>) -> (StatusCode, Json<RuntimeMetricsResponse>) {
    (StatusCode::OK, Json(state.service.metrics_snapshot()))
}

/// Returns build records currently tagged as dead-letter.
async fn dead_letter_builds(
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<DeadLetterBuildsResponse>), StatusCode> {
    let builds = state
        .service
        .list_dead_letter_builds()
        .await
        .map_err(|e| e.status_code())?;
    Ok((StatusCode::OK, Json(DeadLetterBuildsResponse { builds })))
}

/// Creates one job from request payload.
async fn create_job(
    State(state): State<ApiState>,
    Json(payload): Json<CreateJobRequest>,
) -> Result<(StatusCode, Json<CreateJobResponse>), StatusCode> {
    let job = state
        .service
        .create_job(payload)
        .await
        .map_err(|e| e.status_code())?;

    Ok((StatusCode::CREATED, Json(CreateJobResponse { job })))
}

/// Lists all jobs.
async fn list_jobs(
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<ListJobsResponse>), StatusCode> {
    let jobs = state.service.list_jobs().await.map_err(|e| e.status_code())?;
    Ok((StatusCode::OK, Json(ListJobsResponse { jobs })))
}

/// Enqueues one build for the given job id.
async fn run_job(
    Path(id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<RunJobResponse>), StatusCode> {
    let build = state.service.run_job(id).await.map_err(|e| e.status_code())?;

    if state.run_embedded_worker {
        // Embedded mode keeps bootstrap behavior while worker APIs allow external workers.
        let service = state.service.clone();
        tokio::spawn(async move {
            let _ = service.process_next_build().await;
        });
    }

    Ok((StatusCode::CREATED, Json(RunJobResponse { build })))
}

/// Lists all builds.
async fn list_builds(
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<ListBuildsResponse>), StatusCode> {
    let builds = state.service.list_builds().await.map_err(|e| e.status_code())?;
    Ok((StatusCode::OK, Json(ListBuildsResponse { builds })))
}

/// Lists all known workers.
async fn list_workers(
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<ListWorkersResponse>), StatusCode> {
    let workers = state.service.list_workers().map_err(|e| e.status_code())?;
    Ok((StatusCode::OK, Json(ListWorkersResponse { workers })))
}

/// Cancels one build by id.
async fn cancel_build(
    Path(id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<CancelBuildResponse>), StatusCode> {
    let build = state.service.cancel_build(id).await.map_err(|e| e.status_code())?;

    Ok((StatusCode::OK, Json(CancelBuildResponse { build })))
}

/// Claims next available build for one worker.
async fn worker_claim_build(
    Path(worker_id): Path<String>,
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<ClaimBuildResponse>), StatusCode> {
    let build = state
        .service
        .claim_build_for_worker(&worker_id)
        .await
        .map_err(|e| e.status_code())?;
    Ok((StatusCode::OK, Json(ClaimBuildResponse { build })))
}

/// Completes one claimed build for one worker.
async fn worker_complete_build(
    Path((worker_id, id)): Path<(String, Uuid)>,
    State(state): State<ApiState>,
    Json(payload): Json<CompleteBuildRequest>,
) -> Result<(StatusCode, Json<CompleteBuildResponse>), StatusCode> {
    let build = state
        .service
        .complete_build_for_worker(&worker_id, id, payload.status, payload.log_line)
        .await
        .map_err(|e| e.status_code())?;

    Ok((StatusCode::OK, Json(CompleteBuildResponse { build })))
}
