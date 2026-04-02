use async_graphql::{EmptySubscription, Schema};
use axum::{Extension, Router, routing::{get, post}};

use crate::ApiState;
use crate::graphql::{MutationRoot, QueryRoot};
use crate::handlers::{
    check_plugin_authorization,
    cancel_build, create_job, dead_letter_builds, events, graphql_handler, graphql_playground,
    health, ingest_scm_webhook, list_builds, list_jobs, list_plugins, list_workers, live,
    load_plugin, metrics, ready, run_job, run_scm_polling_tick, upsert_plugin_policy,
    upsert_scm_polling_config, upsert_webhook_security_config, worker_claim_build,
    worker_complete_build, execute_plugin, get_plugin_policy, init_plugin,
    list_scm_webhook_rejections, unload_plugin,
};

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
        .route("/plugins", get(list_plugins).post(load_plugin))
        .route("/plugins/policies", get(get_plugin_policy).post(upsert_plugin_policy))
        .route("/workers", get(list_workers))
        .route("/webhooks/scm", post(ingest_scm_webhook))
        .route("/scm/webhook-security/configs", post(upsert_webhook_security_config))
        .route(
            "/scm/webhook-security/rejections",
            get(list_scm_webhook_rejections),
        )
        .route("/scm/polling/configs", post(upsert_scm_polling_config))
        .route("/scm/polling/tick", post(run_scm_polling_tick))
        .route("/jobs/{id}/run", post(run_job))
        .route("/builds/{id}/cancel", post(cancel_build))
        .route("/workers/{worker_id}/claim", post(worker_claim_build))
        .route("/plugins/{name}/init", post(init_plugin))
        .route("/plugins/{name}/execute", post(execute_plugin))
        .route("/plugins/{name}/unload", post(unload_plugin))
        .route(
            "/plugins/{name}/authorize-check",
            post(check_plugin_authorization),
        )
        .route(
            "/workers/{worker_id}/builds/{id}/complete",
            post(worker_complete_build),
        )
        .layer(Extension(graphql_schema))
        .with_state(state)
}
