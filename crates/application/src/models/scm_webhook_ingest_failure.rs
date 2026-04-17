use crate::ApiError;

/// Structured webhook ingestion failure exposed by application use-cases.
pub struct ScmWebhookIngestFailure {
    pub api_error: ApiError,
    pub reason_code: &'static str,
    pub public_message: Option<&'static str>,
}

impl ScmWebhookIngestFailure {
    /// Builds one failure descriptor from a service-level error.
    pub fn from_api_error(api_error: ApiError) -> Self {
        let (reason_code, public_message) = match &api_error {
            ApiError::BadRequest => (
                "invalid_webhook_request",
                Some("webhook request is missing required headers"),
            ),
            ApiError::Unauthorized => (
                "invalid_webhook_signature",
                Some("webhook signature is missing, invalid, or expired"),
            ),
            ApiError::Forbidden => (
                "webhook_forbidden",
                Some("webhook provider/repository/ip is not authorized"),
            ),
            _ => ("webhook_internal_error", None),
        };

        Self {
            api_error,
            reason_code,
            public_message,
        }
    }
}
