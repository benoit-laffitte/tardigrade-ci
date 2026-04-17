use axum::http::StatusCode;
use tardigrade_core::PipelineValidationIssue;

/// Internal service-layer error taxonomy converted to HTTP codes at the edge.
pub enum ApiError {
    BadRequest,
    Unauthorized,
    Forbidden,
    InvalidPipeline {
        message: String,
        details: Option<Vec<PipelineValidationIssue>>,
    },
    NotFound,
    Conflict,
    Internal,
}

impl ApiError {
    /// Maps domain/service errors to stable HTTP status codes.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::InvalidPipeline { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
