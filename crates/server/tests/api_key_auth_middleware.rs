use axum::{
    Json, Router,
    body::Body,
    extract::Extension,
    http::{Request, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tardigrade_api::{ApiAuthContext, ApiAuthStatus, ApiState, build_router};
use tardigrade_scheduler::{adapters::InMemoryScheduler, ports::Scheduler};
use tardigrade_server::auth_middleware::mount_api_key_auth;
use tardigrade_storage::{adapters::InMemoryStorage, ports::Storage};
use tower::ServiceExt;

/// Echoes current request auth context as JSON for middleware verification tests.
async fn auth_echo(context: Option<Extension<ApiAuthContext>>) -> impl IntoResponse {
    let status = context
        .map(|Extension(ctx)| ctx.status)
        .unwrap_or(ApiAuthStatus::Disabled);

    let status_label = match status {
        ApiAuthStatus::Disabled => "disabled",
        ApiAuthStatus::Verified => "verified",
        ApiAuthStatus::Missing => "missing",
        ApiAuthStatus::Invalid => "invalid",
    };

    Json(json!({ "status": status_label }))
}

/// Builds one test router with API key middleware mounted.
fn middleware_test_router(api_key: Option<&str>) -> Router {
    let router = Router::new()
        .route("/graphql", post(auth_echo))
        .route("/status", get(auth_echo));

    mount_api_key_auth(router, api_key.map(ToString::to_string))
}

/// Reads JSON payload from one test response.
async fn read_json(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    serde_json::from_slice(&body).expect("parse json payload")
}

/// Verifies control-plane requests are marked as verified when x-api-key matches configured key.
#[tokio::test]
async fn control_plane_request_with_valid_api_key_is_verified() {
    let app = middleware_test_router(Some("secret-key"));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/graphql")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-api-key", "secret-key")
                .body(Body::from("{}"))
                .expect("build request"),
        )
        .await
        .expect("serve request");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;
    assert_eq!(payload["status"], "verified");
}

/// Verifies control-plane requests are marked as missing when configured key exists but no header is sent.
#[tokio::test]
async fn control_plane_request_without_api_key_is_marked_missing() {
    let app = middleware_test_router(Some("secret-key"));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/graphql")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .expect("build request"),
        )
        .await
        .expect("serve request");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;
    assert_eq!(payload["status"], "missing");
}

/// Verifies control-plane requests are marked invalid when provided key does not match configured key.
#[tokio::test]
async fn control_plane_request_with_invalid_bearer_key_is_marked_invalid() {
    let app = middleware_test_router(Some("secret-key"));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/graphql")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer wrong-key")
                .body(Body::from("{}"))
                .expect("build request"),
        )
        .await
        .expect("serve request");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;
    assert_eq!(payload["status"], "invalid");
}

/// Verifies non-control-plane routes keep disabled status to avoid impacting static/dashboard paths.
#[tokio::test]
async fn non_control_plane_route_keeps_disabled_status() {
    let app = middleware_test_router(Some("secret-key"));

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/status")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("serve request");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;
    assert_eq!(payload["status"], "disabled");
}

/// Builds one API router with auth middleware for end-to-end GraphQL auth checks.
fn secured_graphql_router(api_key: &str) -> Router {
    let storage: Arc<dyn Storage + Send + Sync> = Arc::new(InMemoryStorage::default());
    let scheduler: Arc<dyn Scheduler + Send + Sync> = Arc::new(InMemoryScheduler::default());
    let state = ApiState::with_components("tardigrade-ci-test", storage, scheduler);
    let router = build_router(state);
    mount_api_key_auth(router, Some(api_key.to_string()))
}

/// Builds one GraphQL POST request body.
fn graphql_request(query: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/graphql")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "query": query, "variables": {} }).to_string(),
        ))
        .expect("build graphql request")
}

/// Verifies GraphQL queries remain readable without API key while middleware is enabled.
#[tokio::test]
async fn graphql_query_without_api_key_is_allowed() {
    let app = secured_graphql_router("secret-key");

    let response = app
        .oneshot(graphql_request("query Ready { ready { status } }"))
        .await
        .expect("serve graphql query");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;
    assert!(payload.get("errors").is_none(), "graphql errors: {payload}");
    assert_eq!(payload["data"]["ready"]["status"], "ready");
}

/// Verifies GraphQL mutations are rejected with unauthorized when API key is missing.
#[tokio::test]
async fn graphql_mutation_without_api_key_is_unauthorized() {
    let app = secured_graphql_router("secret-key");

    let response = app
        .oneshot(graphql_request(
            "mutation Tick { run_scm_polling_tick { polled_repositories enqueued_builds } }",
        ))
        .await
        .expect("serve graphql mutation");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;
    assert_eq!(payload["errors"][0]["extensions"]["code"], "unauthorized");
}

/// Verifies GraphQL mutations are rejected with forbidden when API key is invalid.
#[tokio::test]
async fn graphql_mutation_with_invalid_api_key_is_forbidden() {
    let app = secured_graphql_router("secret-key");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/graphql")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-api-key", "wrong-key")
                .body(Body::from(
                    json!({
                        "query": "mutation Tick { run_scm_polling_tick { polled_repositories enqueued_builds } }",
                        "variables": {}
                    })
                    .to_string(),
                ))
                .expect("build graphql request"),
        )
        .await
        .expect("serve graphql mutation");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;
    assert_eq!(payload["errors"][0]["extensions"]["code"], "forbidden");
}
