use async_graphql::{EmptySubscription, Schema};
use axum::{Extension, Router, routing::{get, post}};

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

