use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    ApiState, ClaimBuildResponse, CompleteBuildRequest, CompleteBuildResponse,
    ListWorkersResponse,
};

/// Lists all known workers.
pub(crate) async fn list_workers(
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<ListWorkersResponse>), StatusCode> {
    let workers = state.service.list_workers().map_err(|e| e.status_code())?;
    Ok((StatusCode::OK, Json(ListWorkersResponse { workers })))
}

/// Claims next available build for one worker.
pub(crate) async fn worker_claim_build(
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
pub(crate) async fn worker_complete_build(
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
