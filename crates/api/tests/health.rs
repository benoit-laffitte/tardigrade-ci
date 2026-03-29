use axum::http::StatusCode;
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/health")
                .body(axum::body::Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn live_endpoint_returns_ok() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/live")
                .body(axum::body::Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ready_endpoint_returns_ok() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/ready")
                .body(axum::body::Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}
