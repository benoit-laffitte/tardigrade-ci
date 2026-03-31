use axum::{
    Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use crate::{
    ApiError, ApiErrorResponse, ApiState, ScmPollingTickResponse, ScmWebhookAcceptedResponse,
    UpsertScmPollingConfigRequest,
};

/// Ingests one SCM webhook with strict signature, replay-window, and IP allowlist checks.
pub(crate) async fn ingest_scm_webhook(
    State(state): State<ApiState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    state.service.record_scm_webhook_received();

    match state.service.ingest_scm_webhook(&headers, &body).await {
        Ok(()) => {
            state.service.record_scm_webhook_accepted();
            (
                StatusCode::ACCEPTED,
                Json(ScmWebhookAcceptedResponse {
                    status: "accepted".to_string(),
                }),
            )
                .into_response()
        }
        Err(ApiError::BadRequest) => {
            state.service.record_scm_webhook_rejected();
            (
                StatusCode::BAD_REQUEST,
                Json(ApiErrorResponse {
                    code: "invalid_webhook_request".to_string(),
                    message: "webhook request is missing required headers".to_string(),
                    details: None,
                }),
            )
                .into_response()
        }
        Err(ApiError::Unauthorized) => {
            state.service.record_scm_webhook_rejected();
            (
                StatusCode::UNAUTHORIZED,
                Json(ApiErrorResponse {
                    code: "invalid_webhook_signature".to_string(),
                    message: "webhook signature is missing, invalid, or expired".to_string(),
                    details: None,
                }),
            )
                .into_response()
        }
        Err(ApiError::Forbidden) => {
            state.service.record_scm_webhook_rejected();
            (
                StatusCode::FORBIDDEN,
                Json(ApiErrorResponse {
                    code: "webhook_forbidden".to_string(),
                    message: "webhook provider/repository/ip is not authorized".to_string(),
                    details: None,
                }),
            )
                .into_response()
        }
        Err(err) => {
            state.service.record_scm_webhook_rejected();
            err.status_code().into_response()
        }
    }
}

/// Upserts SCM polling configuration for one repository/provider.
pub(crate) async fn upsert_scm_polling_config(
    State(state): State<ApiState>,
    Json(payload): Json<UpsertScmPollingConfigRequest>,
) -> Result<StatusCode, StatusCode> {
    state.upsert_scm_polling_config(payload).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Runs one SCM polling tick immediately.
pub(crate) async fn run_scm_polling_tick(
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<ScmPollingTickResponse>), StatusCode> {
    let result = state
        .service
        .run_scm_polling_tick()
        .await
        .map_err(|e| e.status_code())?;
    Ok((StatusCode::OK, Json(result)))
}
