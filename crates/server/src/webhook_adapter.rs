use axum::{
    Router, body::Bytes, extract::State, http::HeaderMap, response::Response, routing::post,
};
use tardigrade_api::ApiState;

/// Mounts the native SCM webhook adapter route on top of the GraphQL control plane.
pub fn mount_webhook_adapter(router: Router, state: ApiState) -> Router {
    router.merge(
        Router::new()
            .route("/webhooks/scm", post(ingest_scm_webhook))
            .with_state(state),
    )
}

/// Accepts one native SCM webhook and delegates processing to the GraphQL-only API state.
async fn ingest_scm_webhook(
    State(state): State<ApiState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    state.ingest_scm_webhook_http(headers, &body).await
}
