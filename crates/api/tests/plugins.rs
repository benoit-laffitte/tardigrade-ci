use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde_json::Value;
use tower::ServiceExt;

use tardigrade_api::{ListPluginsResponse, PluginActionResponse};

/// Reads one JSON payload from an axum response.
async fn read_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&body).expect("valid json")
}

/// Builds a JSON POST request with optional body payload.
fn json_post(uri: &str, body: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method("POST").uri(uri);
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }

    builder
        .body(Body::from(body.unwrap_or("{}")))
        .expect("valid request")
}

#[tokio::test]
/// Verifies plugin lifecycle happy path through admin endpoints.
async fn plugin_admin_lifecycle_happy_path() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let list_before = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/plugins")
                .body(Body::empty())
                .expect("valid list request"),
        )
        .await
        .expect("list response");

    assert_eq!(list_before.status(), StatusCode::OK);
    let before_payload: ListPluginsResponse =
        serde_json::from_value(read_json(list_before).await).expect("plugins payload");
    assert!(before_payload.plugins.is_empty());

    let load_response = app
        .clone()
        .oneshot(json_post(
            "/plugins",
            Some(r#"{"name":"net-diagnostics"}"#),
        ))
        .await
        .expect("load response");

    assert_eq!(load_response.status(), StatusCode::CREATED);
    let load_payload: PluginActionResponse =
        serde_json::from_value(read_json(load_response).await).expect("load payload");
    assert_eq!(load_payload.status, "loaded");
    assert_eq!(load_payload.plugin.name, "net-diagnostics");
    assert_eq!(load_payload.plugin.state, "Loaded");
    assert_eq!(load_payload.plugin.capabilities, vec!["network".to_string()]);

    let init_response = app
        .clone()
        .oneshot(json_post("/plugins/net-diagnostics/init", None))
        .await
        .expect("init response");

    assert_eq!(init_response.status(), StatusCode::OK);
    let init_payload: PluginActionResponse =
        serde_json::from_value(read_json(init_response).await).expect("init payload");
    assert_eq!(init_payload.status, "initialized");
    assert_eq!(init_payload.plugin.state, "Initialized");

    let execute_response = app
        .clone()
        .oneshot(json_post("/plugins/net-diagnostics/execute", None))
        .await
        .expect("execute response");

    assert_eq!(execute_response.status(), StatusCode::OK);
    let execute_payload: PluginActionResponse =
        serde_json::from_value(read_json(execute_response).await).expect("execute payload");
    assert_eq!(execute_payload.status, "executed");

    let unload_response = app
        .clone()
        .oneshot(json_post("/plugins/net-diagnostics/unload", None))
        .await
        .expect("unload response");

    assert_eq!(unload_response.status(), StatusCode::OK);
    let unload_payload: PluginActionResponse =
        serde_json::from_value(read_json(unload_response).await).expect("unload payload");
    assert_eq!(unload_payload.status, "unloaded");
    assert_eq!(unload_payload.plugin.state, "Unloaded");

    let list_after = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/plugins")
                .body(Body::empty())
                .expect("valid list request"),
        )
        .await
        .expect("list response");

    assert_eq!(list_after.status(), StatusCode::OK);
    let after_payload: ListPluginsResponse =
        serde_json::from_value(read_json(list_after).await).expect("plugins payload");
    assert_eq!(after_payload.plugins.len(), 1);
    assert_eq!(after_payload.plugins[0].name, "net-diagnostics");
    assert_eq!(after_payload.plugins[0].state, "Unloaded");
}

#[tokio::test]
/// Verifies lifecycle invalid transition returns actionable conflict payload.
async fn plugin_admin_execute_before_init_returns_invalid_state() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let _ = app
        .clone()
        .oneshot(json_post("/plugins", Some(r#"{"name":"fs-audit"}"#)))
        .await
        .expect("load response");

    let execute_response = app
        .oneshot(json_post("/plugins/fs-audit/execute", None))
        .await
        .expect("execute response");

    assert_eq!(execute_response.status(), StatusCode::CONFLICT);
    let payload = read_json(execute_response).await;
    assert_eq!(payload["code"], "plugin_invalid_state");
}

#[tokio::test]
/// Verifies panic-safe execution mapping is exposed as explicit API error.
async fn plugin_admin_execute_panic_probe_reports_contained_panic() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let _ = app
        .clone()
        .oneshot(json_post("/plugins", Some(r#"{"name":"panic-probe"}"#)))
        .await
        .expect("load response");

    let _ = app
        .clone()
        .oneshot(json_post("/plugins/panic-probe/init", None))
        .await
        .expect("init response");

    let execute_response = app
        .oneshot(json_post("/plugins/panic-probe/execute", None))
        .await
        .expect("execute response");

    assert_eq!(execute_response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let payload = read_json(execute_response).await;
    assert_eq!(payload["code"], "plugin_execution_panicked");
}
