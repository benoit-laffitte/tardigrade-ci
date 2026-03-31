use axum::{Json, extract::State, http::StatusCode};

use crate::{ApiState, HealthResponse, LiveResponse, ReadyResponse};

/// Returns service identity and basic health signal.
pub(crate) async fn health(State(state): State<ApiState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: state.service_name,
    })
}

/// Returns process liveness probe response.
pub(crate) async fn live() -> Json<LiveResponse> {
    Json(LiveResponse { status: "alive" })
}

/// Returns readiness probe response after dependency checks.
pub(crate) async fn ready(
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<ReadyResponse>), StatusCode> {
    state
        .service
        .is_ready()
        .await
        .map_err(|e| e.status_code())?;
    Ok((StatusCode::OK, Json(ReadyResponse { status: "ready" })))
}
