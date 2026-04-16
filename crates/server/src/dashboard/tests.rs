use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use std::fs;
use std::path::PathBuf;
use std::sync::Once;
use tower::ServiceExt;

use super::{mount_dashboard_assets, resolve_web_root};

/// Ensures canonical target/public assets exist for handler tests.
fn ensure_dashboard_test_assets() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = crate_root
            .parent()
            .and_then(|path| path.parent())
            .map(|path| path.to_path_buf())
            .unwrap_or_else(|| crate_root.clone());
        let target_root = workspace_root.join("target").join("public");

        fs::create_dir_all(&target_root).expect("create target/public for dashboard tests");

        // Keep test assets tiny and deterministic to avoid coupling tests to frontend build outputs.
        fs::write(
            target_root.join("index.html"),
            "<!doctype html><html><head></head><body>test dashboard</body></html>",
        )
        .expect("write dashboard index fixture");
        fs::write(
            target_root.join("app.js"),
            "console.log('tardigrade dashboard test');",
        )
        .expect("write dashboard js fixture");
        fs::write(
            target_root.join("styles.css"),
            "body { background: #fff; color: #111; }",
        )
        .expect("write dashboard css fixture");
        fs::write(
            target_root.join("tardigrade-logo.png"),
            // Minimal PNG signature + one byte payload is enough for byte-oriented handler checks.
            [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00],
        )
        .expect("write dashboard logo fixture");
    });
}

/// Builds a router exposing dashboard assets through the mounted directory service.
fn dashboard_test_router() -> axum::Router {
    mount_dashboard_assets(axum::Router::new(), resolve_web_root(None))
}

/// Verifies dashboard root path returns an HTML payload.
#[tokio::test]
async fn dashboard_root_returns_html_payload() {
    ensure_dashboard_test_assets();
    let response = dashboard_test_router()
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("serve root request");
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let html = String::from_utf8(body.to_vec()).expect("utf8 html");
    assert!(html.contains("<html"));
}

/// Verifies javascript asset path sets expected content type.
#[tokio::test]
async fn javascript_asset_sets_javascript_content_type() {
    ensure_dashboard_test_assets();
    let response = dashboard_test_router()
        .oneshot(
            Request::builder()
                .uri("/app.js")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("serve javascript request");
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert_eq!(content_type, "text/javascript");
}

/// Verifies stylesheet asset path sets expected content type.
#[tokio::test]
async fn stylesheet_asset_sets_css_content_type() {
    ensure_dashboard_test_assets();
    let response = dashboard_test_router()
        .oneshot(
            Request::builder()
                .uri("/styles.css")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("serve stylesheet request");
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert_eq!(content_type, "text/css");
}

/// Verifies logo asset path sets png content type and returns bytes.
#[tokio::test]
async fn logo_asset_sets_png_content_type() {
    ensure_dashboard_test_assets();
    let response = dashboard_test_router()
        .oneshot(
            Request::builder()
                .uri("/tardigrade-logo.png")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("serve logo request");
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert_eq!(content_type, "image/png");

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read logo body");
    assert!(!body.is_empty());
}

/// Verifies unknown dashboard asset paths return not found.
#[tokio::test]
async fn unknown_dashboard_asset_returns_not_found() {
    ensure_dashboard_test_assets();
    let response = dashboard_test_router()
        .oneshot(
            Request::builder()
                .uri("/missing-resource.js")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("serve missing asset request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
