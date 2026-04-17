use std::sync::Arc;

use axum::{
    Router,
    extract::Request,
    extract::State,
    http::header,
    middleware::{self, Next},
    response::Response,
};
use tardigrade_api::{ApiAuthContext, ApiAuthStatus};

/// Header name accepted for direct API key authentication.
const API_KEY_HEADER: &str = "x-api-key";

/// Shared middleware state carrying optional API key verifier.
struct ApiKeyAuthState {
    expected_key: Option<String>,
}

/// Mounts API key extraction/verification middleware on control-plane router branches.
pub fn mount_api_key_auth(router: Router, configured_api_key: Option<String>) -> Router {
    let expected_key = configured_api_key.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let state = Arc::new(ApiKeyAuthState { expected_key });
    router.layer(middleware::from_fn_with_state(
        state,
        api_key_auth_middleware,
    ))
}

/// Extracts one API key candidate from accepted auth headers.
fn extract_api_key_candidate(request: &Request) -> Option<String> {
    if let Some(value) = request
        .headers()
        .get(API_KEY_HEADER)
        .and_then(|raw| raw.to_str().ok())
    {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let authorization = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|raw| raw.to_str().ok())?;

    let bearer_prefix = "Bearer ";
    if let Some(token) = authorization.strip_prefix(bearer_prefix) {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

/// Applies API key verification on control-plane requests and injects auth context for downstream handlers.
async fn api_key_auth_middleware(
    State(state): State<Arc<ApiKeyAuthState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    let applies_to_control_plane = path == "/graphql" || path == "/webhooks/scm";

    let status = if !applies_to_control_plane {
        ApiAuthStatus::Disabled
    } else if let Some(expected_key) = state.expected_key.as_ref() {
        match extract_api_key_candidate(&request) {
            Some(candidate) if candidate == *expected_key => ApiAuthStatus::Verified,
            Some(_) => ApiAuthStatus::Invalid,
            None => ApiAuthStatus::Missing,
        }
    } else {
        ApiAuthStatus::Disabled
    };

    request.extensions_mut().insert(ApiAuthContext {
        status: status.clone(),
    });

    next.run(request).await
}
