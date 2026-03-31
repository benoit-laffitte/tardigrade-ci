use axum::response::{Html, IntoResponse};

use super::{APP_JS, INDEX_HTML, STYLES_CSS};

/// Serves dashboard index HTML.
pub async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

/// Serves dashboard javascript with explicit content type.
pub async fn app_js() -> impl IntoResponse {
    (
        [("content-type", "application/javascript; charset=utf-8")],
        APP_JS,
    )
}

/// Serves dashboard stylesheet with explicit content type.
pub async fn styles_css() -> impl IntoResponse {
    ([("content-type", "text/css; charset=utf-8")], STYLES_CSS)
}
