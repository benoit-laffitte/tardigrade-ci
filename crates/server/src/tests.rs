use super::{
    RuntimeMode, app_js, index, load_runtime_mode_from_config, parse_runtime_mode_from_toml,
    styles_css,
};
use axum::{body::to_bytes, response::IntoResponse};
use std::time::{SystemTime, UNIX_EPOCH};

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

#[test]
/// Parses explicit prod mode from TOML runtime section.
fn parse_runtime_mode_reads_prod_value() {
    let raw = r#"
[runtime]
mode = "prod"
"#;

    let mode = parse_runtime_mode_from_toml(raw).expect("parse runtime mode");
    assert_eq!(mode, RuntimeMode::Prod);
}

#[test]
/// Defaults to dev mode when runtime section is omitted.
fn parse_runtime_mode_defaults_to_dev_when_runtime_missing() {
    let raw = r#"
[server]
bind = "127.0.0.1:8080"
"#;

    let mode = parse_runtime_mode_from_toml(raw).expect("parse runtime mode");
    assert_eq!(mode, RuntimeMode::Dev);
}

#[test]
/// Missing config file path falls back to dev mode for bootstrap ergonomics.
fn load_runtime_mode_defaults_to_dev_when_file_is_missing() {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("unix epoch")
        .as_nanos();
    let missing_path = format!("/tmp/tardigrade-missing-config-{unique_suffix}.toml");
    let mode = load_runtime_mode_from_config(&missing_path).expect("load runtime mode");
    assert_eq!(mode, RuntimeMode::Dev);
}
