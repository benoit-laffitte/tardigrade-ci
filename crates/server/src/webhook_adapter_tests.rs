use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tardigrade_api::{ApiState, build_router};
use tower::ServiceExt;

use crate::webhook_adapter::mount_webhook_adapter;

/// Builds the plain API router without the native webhook adapter.
fn api_only_router() -> axum::Router {
    build_router(ApiState::new("test-service"))
}

/// Builds the server router with the native webhook adapter mounted on top.
fn api_with_webhook_adapter_router() -> axum::Router {
    let state = ApiState::new("test-service");
    mount_webhook_adapter(build_router(state.clone()), state)
}

/// Verifies the API crate alone no longer exposes the native SCM webhook route.
#[tokio::test]
async fn api_router_does_not_expose_native_webhook_route() {
    let response = api_only_router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks/scm")
                .body(Body::from("{}"))
                .expect("build webhook request"),
        )
        .await
        .expect("serve api-only webhook request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Verifies the dedicated server adapter accepts the route and delegates validation to API state.
#[tokio::test]
async fn webhook_adapter_route_returns_structured_bad_request() {
    let response = api_with_webhook_adapter_router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks/scm")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("build webhook request"),
        )
        .await
        .expect("serve webhook adapter request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read webhook adapter body");
    let payload = String::from_utf8(body.to_vec()).expect("decode webhook adapter body");
    assert!(payload.contains("invalid_webhook_request"));
}
