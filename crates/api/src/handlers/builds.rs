use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{ApiState, CancelBuildResponse, ListBuildsResponse};

/// Lists all builds.
pub(crate) async fn list_builds(
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<ListBuildsResponse>), StatusCode> {
    let builds = state
        .service
        .list_builds()
        .await
        .map_err(|e| e.status_code())?;
    Ok((StatusCode::OK, Json(ListBuildsResponse { builds })))
}

/// Cancels one build by id.
pub(crate) async fn cancel_build(
    Path(id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<CancelBuildResponse>), StatusCode> {
    let build = state
        .service
        .cancel_build(id)
        .await
        .map_err(|e| e.status_code())?;

    Ok((StatusCode::OK, Json(CancelBuildResponse { build })))
}
