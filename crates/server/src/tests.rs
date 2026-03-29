use super::{app_js, index, styles_css, tardigrade_logo_png};
use axum::{
    body::{Body, to_bytes},
    response::IntoResponse,
};

#[tokio::test]
async fn index_handler_returns_html_payload() {
    let response = index().await.into_response();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let html = String::from_utf8(body.to_vec()).expect("utf8 html");
    assert!(html.contains("<html"));
}

#[tokio::test]
async fn app_js_handler_sets_javascript_content_type() {
    let response = app_js().await.into_response();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert_eq!(content_type, "application/javascript; charset=utf-8");
}

#[tokio::test]
async fn styles_handler_sets_css_content_type() {
    let response = styles_css().await.into_response();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert_eq!(content_type, "text/css; charset=utf-8");
}

#[tokio::test]
async fn logo_handler_returns_png_content_type_and_body() {
    let response = tardigrade_logo_png().await.into_response();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert_eq!(content_type, "image/png");

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    assert!(!body.is_empty());
    let _ = Body::from(body);
}
