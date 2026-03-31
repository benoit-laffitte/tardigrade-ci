use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    ApiError, ApiErrorResponse, ApiState, CreateJobRequest, CreateJobResponse, ListJobsResponse,
    RunJobResponse,
};

/// Creates one job from request payload.
pub(crate) async fn create_job(
    State(state): State<ApiState>,
    Json(payload): Json<CreateJobRequest>,
) -> Response {
    match state.service.create_job(payload).await {
        Ok(job) => (StatusCode::CREATED, Json(CreateJobResponse { job })).into_response(),
        Err(ApiError::InvalidPipeline { message, details }) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiErrorResponse {
                code: "invalid_pipeline".to_string(),
                message,
                details,
            }),
        )
            .into_response(),
        Err(err) => err.status_code().into_response(),
    }
}

/// Lists all jobs.
pub(crate) async fn list_jobs(
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<ListJobsResponse>), StatusCode> {
    let jobs = state
        .service
        .list_jobs()
        .await
        .map_err(|e| e.status_code())?;
    Ok((StatusCode::OK, Json(ListJobsResponse { jobs })))
}

/// Enqueues one build for the given job id.
pub(crate) async fn run_job(
    Path(id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<RunJobResponse>), StatusCode> {
    let build = state
        .service
        .run_job(id)
        .await
        .map_err(|e| e.status_code())?;

    if state.run_embedded_worker {
        // Embedded mode keeps bootstrap behavior while worker APIs allow external workers.
        let service = state.service.clone();
        tokio::spawn(async move {
            let _ = service.process_next_build().await;
        });
    }

    Ok((StatusCode::CREATED, Json(RunJobResponse { build })))
}
