use axum::{
    Json,
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    ApiError, ApiErrorResponse, ApiState, ListScmWebhookRejectionsResponse, ScmPollingTickResponse,
    ScmWebhookAcceptedResponse, UpsertScmPollingConfigRequest, UpsertWebhookSecurityConfigRequest,
};

/// Query payload for webhook rejection diagnostics filtering.
#[derive(Debug, Deserialize)]
pub(crate) struct ScmWebhookRejectionsQuery {
    pub provider: Option<String>,
    pub repository_url: Option<String>,
    pub limit: Option<usize>,
}

/// Ingests one SCM webhook with strict signature, replay-window, and IP allowlist checks.
pub(crate) async fn ingest_scm_webhook(
    State(state): State<ApiState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let provider = optional_header_value(&headers, "x-scm-provider");
    let repository_url = optional_header_value(&headers, "x-scm-repository");

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
            state.service.record_scm_webhook_rejection(
                "invalid_webhook_request",
                provider.as_deref(),
                repository_url.as_deref(),
            );
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
            state.service.record_scm_webhook_rejection(
                "invalid_webhook_signature",
                provider.as_deref(),
                repository_url.as_deref(),
            );
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
            state.service.record_scm_webhook_rejection(
                "webhook_forbidden",
                provider.as_deref(),
                repository_url.as_deref(),
            );
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
            state.service.record_scm_webhook_rejection(
                "webhook_internal_error",
                provider.as_deref(),
                repository_url.as_deref(),
            );
            err.status_code().into_response()
        }
    }
}

/// Returns recent webhook rejection diagnostics with optional provider/repository filters.
pub(crate) async fn list_scm_webhook_rejections(
    Query(query): Query<ScmWebhookRejectionsQuery>,
    State(state): State<ApiState>,
) -> (StatusCode, Json<ListScmWebhookRejectionsResponse>) {
    let rejections = state.service.list_scm_webhook_rejections(
        query.provider.as_deref(),
        query.repository_url.as_deref(),
        query.limit.unwrap_or(20),
    );
    (
        StatusCode::OK,
        Json(ListScmWebhookRejectionsResponse { rejections }),
    )
}

/// Upserts SCM polling configuration for one repository/provider.
pub(crate) async fn upsert_scm_polling_config(
    State(state): State<ApiState>,
    Json(payload): Json<UpsertScmPollingConfigRequest>,
) -> Result<StatusCode, StatusCode> {
    state.upsert_scm_polling_config(payload).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Upserts webhook security configuration for one repository/provider.
pub(crate) async fn upsert_webhook_security_config(
    State(state): State<ApiState>,
    Json(payload): Json<UpsertWebhookSecurityConfigRequest>,
) -> Result<StatusCode, StatusCode> {
    state.upsert_webhook_security_config(payload).await?;
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

/// Reads one optional string header value and normalizes invalid values to None.
fn optional_header_value(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get(key)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}
