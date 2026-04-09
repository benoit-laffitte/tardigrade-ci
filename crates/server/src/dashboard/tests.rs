use axum::{body::to_bytes, response::IntoResponse};
use std::fs;
use std::path::PathBuf;
use std::sync::Once;

use super::{app_js, index, styles_css, tardigrade_logo_png};

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
        let source_root = crate_root.join("static");
        let target_root = workspace_root.join("target").join("public");

        fs::create_dir_all(&target_root).expect("create target/public for dashboard tests");
        for file_name in ["index.html", "app.js", "styles.css", "tardigrade-logo.png"] {
            let source = source_root.join(file_name);
            let target = target_root.join(file_name);
            fs::copy(&source, &target).unwrap_or_else(|error| {
                panic!(
                    "copy dashboard test asset {} -> {} failed: {}",
                    source.display(),
                    target.display(),
                    error
                )
            });
        }
    });
}

/// Verifies dashboard index handler returns an HTML payload.
#[tokio::test]
async fn index_handler_returns_html_payload() {
    ensure_dashboard_test_assets();
    let response = index().await.into_response();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let html = String::from_utf8(body.to_vec()).expect("utf8 html");
    assert!(html.contains("<html"));
}

/// Verifies javascript asset handler sets expected content type.
#[tokio::test]
async fn app_js_handler_sets_javascript_content_type() {
    ensure_dashboard_test_assets();
    let response = app_js().await.into_response();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert_eq!(content_type, "application/javascript; charset=utf-8");
}

/// Verifies stylesheet asset handler sets expected content type.
#[tokio::test]
async fn styles_handler_sets_css_content_type() {
    ensure_dashboard_test_assets();
    let response = styles_css().await.into_response();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert_eq!(content_type, "text/css; charset=utf-8");
}

/// Verifies logo asset handler sets png content type and returns bytes.
#[tokio::test]
async fn logo_handler_sets_png_content_type() {
    ensure_dashboard_test_assets();
    let response = tardigrade_logo_png().await.into_response();
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
