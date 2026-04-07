use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use std::fs;
use tracing::warn;

use super::assets::resolve_asset_path;

/// Serves dashboard index HTML.
pub async fn index() -> impl IntoResponse {
    let path = resolve_asset_path("index.html");
    match fs::read_to_string(&path) {
        Ok(html) => Html(html).into_response(),
        Err(error) => {
            warn!(path = %path.display(), %error, "failed to load dashboard index");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "dashboard index not available",
            )
                .into_response()
        }
    }
}

/// Serves dashboard javascript with explicit content type.
pub async fn app_js() -> impl IntoResponse {
    let path = resolve_asset_path("app.js");
    match fs::read_to_string(&path) {
        Ok(script) => (
            [("content-type", "application/javascript; charset=utf-8")],
            script,
        )
            .into_response(),
        Err(error) => {
            warn!(path = %path.display(), %error, "failed to load dashboard app.js");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "dashboard app.js not available",
            )
                .into_response()
        }
    }
}

/// Serves dashboard stylesheet with explicit content type.
pub async fn styles_css() -> impl IntoResponse {
    let path = resolve_asset_path("styles.css");
    match fs::read_to_string(&path) {
        Ok(stylesheet) => {
            ([("content-type", "text/css; charset=utf-8")], stylesheet).into_response()
        }
        Err(error) => {
            warn!(path = %path.display(), %error, "failed to load dashboard styles.css");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "dashboard styles.css not available",
            )
                .into_response()
        }
    }
}

/// Serves the embedded dashboard logo with explicit content type.
pub async fn tardigrade_logo_png() -> impl IntoResponse {
    let path = resolve_asset_path("tardigrade-logo.png");
    match fs::read(&path) {
        Ok(bytes) => ([("content-type", "image/png")], bytes).into_response(),
        Err(error) => {
            warn!(path = %path.display(), %error, "failed to load dashboard tardigrade-logo.png");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "dashboard logo not available",
            )
                .into_response()
        }
    }
}
