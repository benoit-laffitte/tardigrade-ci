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
use tardigrade_application::ServiceSettings;
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
    secured_graphql_router_with_settings(api_key, ServiceSettings::default())
}

/// Builds one API router with auth middleware and explicit reliability settings.
fn secured_graphql_router_with_settings(api_key: &str, settings: ServiceSettings) -> Router {
    let storage: Arc<dyn Storage + Send + Sync> = Arc::new(InMemoryStorage::default());
    let scheduler: Arc<dyn Scheduler + Send + Sync> = Arc::new(InMemoryScheduler::default());
    let state =
        ApiState::with_components_and_settings("tardigrade-ci-test", storage, scheduler, settings);
    let router = build_router(state);
    mount_api_key_auth(router, Some(api_key.to_string()))
}

/// Builds one GraphQL POST request body with optional API key for middleware checks.
fn graphql_request(query: &str, variables: Value, api_key: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(key) = api_key {
        builder = builder.header("x-api-key", key);
    }

    builder
        .body(Body::from(
            json!({ "query": query, "variables": variables }).to_string(),
        ))
        .expect("build graphql request")
}

/// Verifies GraphQL queries remain readable without API key while middleware is enabled.
#[tokio::test]
async fn graphql_query_without_api_key_is_allowed() {
    let app = secured_graphql_router("secret-key");

    let response = app
        .oneshot(graphql_request(
            "query Ready { ready { status } }",
            json!({}),
            None,
        ))
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
            json!({}),
            None,
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
        .oneshot(graphql_request(
            "mutation Tick { run_scm_polling_tick { polled_repositories enqueued_builds } }",
            json!({}),
            Some("wrong-key"),
        ))
        .await
        .expect("serve graphql mutation");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;
    assert_eq!(payload["errors"][0]["extensions"]["code"], "forbidden");
}

/// Verifies ownership conflicts are surfaced when another worker completes a claimed build.
#[tokio::test]
async fn graphql_worker_complete_with_wrong_owner_reports_conflict_and_metric() {
    let app = secured_graphql_router_with_settings(
        "secret-key",
        ServiceSettings {
            max_retries: 0,
            retry_backoff_ms: 1,
            ..ServiceSettings::default()
        },
    );

    let create_response = app
        .clone()
        .oneshot(graphql_request(
            r#"
            mutation Create($input: GqlCreateJobInput!) {
              create_job(input: $input) { id }
            }
            "#,
            json!({
                "input": {
                    "name": "ownership-conflict",
                    "repository_url": "https://example.com/repo.git",
                    "pipeline_path": "pipeline.yml"
                }
            }),
            Some("secret-key"),
        ))
        .await
        .expect("serve create request");
    let create_payload = read_json(create_response).await;
    let job_id = create_payload["data"]["create_job"]["id"]
        .as_str()
        .expect("job id")
        .to_string();

    let run_response = app
        .clone()
        .oneshot(graphql_request(
            r#"
            mutation Run($jobId: ID!) {
              run_job(jobId: $jobId) { id }
            }
            "#,
            json!({ "jobId": job_id }),
            Some("secret-key"),
        ))
        .await
        .expect("serve run request");
    let run_payload = read_json(run_response).await;
    let build_id = run_payload["data"]["run_job"]["id"]
        .as_str()
        .expect("build id")
        .to_string();

    let claim_response = app
        .clone()
        .oneshot(graphql_request(
            r#"
            mutation Claim($workerId: String!) {
              worker_claim_build(workerId: $workerId) { id status }
            }
            "#,
            json!({ "workerId": "worker-a" }),
            Some("secret-key"),
        ))
        .await
        .expect("serve claim request");
    let claim_payload = read_json(claim_response).await;
    assert_eq!(claim_payload["data"]["worker_claim_build"]["id"], build_id);

    let complete_conflict_response = app
        .clone()
        .oneshot(graphql_request(
            r#"
            mutation Complete($workerId: String!, $buildId: ID!, $status: GqlWorkerBuildStatus!) {
              worker_complete_build(workerId: $workerId, buildId: $buildId, status: $status) { id status }
            }
            "#,
            json!({
                "workerId": "worker-b",
                "buildId": build_id,
                "status": "FAILED"
            }),
            Some("secret-key"),
        ))
        .await
        .expect("serve conflict completion request");
    let complete_conflict_payload = read_json(complete_conflict_response).await;
    let message = complete_conflict_payload["errors"][0]["message"]
        .as_str()
        .expect("error message");
    assert!(message.contains("status 409"));

    let metrics_response = app
        .oneshot(graphql_request(
            "query Metrics { metrics { ownership_conflicts_total } }",
            json!({}),
            None,
        ))
        .await
        .expect("serve metrics request");
    let metrics_payload = read_json(metrics_response).await;
    assert_eq!(
        metrics_payload["data"]["metrics"]["ownership_conflicts_total"],
        1
    );
}

/// Verifies cancellation interactions keep terminal canceled status on late worker completion.
#[tokio::test]
async fn graphql_cancelled_build_stays_canceled_after_late_completion() {
    let app = secured_graphql_router_with_settings(
        "secret-key",
        ServiceSettings {
            max_retries: 0,
            retry_backoff_ms: 1,
            ..ServiceSettings::default()
        },
    );

    let create_payload = read_json(
        app.clone()
            .oneshot(graphql_request(
                r#"
                mutation Create($input: GqlCreateJobInput!) {
                  create_job(input: $input) { id }
                }
                "#,
                json!({
                    "input": {
                        "name": "cancel-interaction",
                        "repository_url": "https://example.com/repo.git",
                        "pipeline_path": "pipeline.yml"
                    }
                }),
                Some("secret-key"),
            ))
            .await
            .expect("create response"),
    )
    .await;
    let job_id = create_payload["data"]["create_job"]["id"]
        .as_str()
        .expect("job id")
        .to_string();

    let run_payload = read_json(
        app.clone()
            .oneshot(graphql_request(
                r#"
                mutation Run($jobId: ID!) {
                  run_job(jobId: $jobId) { id }
                }
                "#,
                json!({ "jobId": job_id }),
                Some("secret-key"),
            ))
            .await
            .expect("run response"),
    )
    .await;
    let build_id = run_payload["data"]["run_job"]["id"]
        .as_str()
        .expect("build id")
        .to_string();

    let _claim_payload = read_json(
        app.clone()
            .oneshot(graphql_request(
                r#"
                mutation Claim($workerId: String!) {
                  worker_claim_build(workerId: $workerId) { id status }
                }
                "#,
                json!({ "workerId": "worker-a" }),
                Some("secret-key"),
            ))
            .await
            .expect("claim response"),
    )
    .await;

    let cancel_payload = read_json(
        app.clone()
            .oneshot(graphql_request(
                r#"
                mutation Cancel($buildId: ID!) {
                  cancel_build(buildId: $buildId) { id status }
                }
                "#,
                json!({ "buildId": build_id }),
                Some("secret-key"),
            ))
            .await
            .expect("cancel response"),
    )
    .await;
    assert_eq!(cancel_payload["data"]["cancel_build"]["status"], "CANCELED");

    let late_completion_payload = read_json(
        app.clone()
            .oneshot(graphql_request(
                r#"
                mutation Complete($workerId: String!, $buildId: ID!, $status: GqlWorkerBuildStatus!) {
                  worker_complete_build(workerId: $workerId, buildId: $buildId, status: $status) { id status }
                }
                "#,
                json!({
                    "workerId": "worker-a",
                    "buildId": build_id,
                    "status": "FAILED"
                }),
                Some("secret-key"),
            ))
            .await
            .expect("late completion response"),
    )
    .await;
    assert_eq!(
        late_completion_payload["data"]["worker_complete_build"]["status"],
        "CANCELED"
    );
}

/// Verifies immediate dead-letter placement when retries are exhausted.
#[tokio::test]
async fn graphql_failed_build_enters_dead_letter_when_max_retries_is_zero() {
    let app = secured_graphql_router_with_settings(
        "secret-key",
        ServiceSettings {
            max_retries: 0,
            retry_backoff_ms: 1,
            ..ServiceSettings::default()
        },
    );

    let create_payload = read_json(
        app.clone()
            .oneshot(graphql_request(
                r#"
                mutation Create($input: GqlCreateJobInput!) {
                  create_job(input: $input) { id }
                }
                "#,
                json!({
                    "input": {
                        "name": "dead-letter",
                        "repository_url": "https://example.com/repo.git",
                        "pipeline_path": "pipeline.yml"
                    }
                }),
                Some("secret-key"),
            ))
            .await
            .expect("create response"),
    )
    .await;
    let job_id = create_payload["data"]["create_job"]["id"]
        .as_str()
        .expect("job id")
        .to_string();

    let run_payload = read_json(
        app.clone()
            .oneshot(graphql_request(
                r#"
                mutation Run($jobId: ID!) {
                  run_job(jobId: $jobId) { id }
                }
                "#,
                json!({ "jobId": job_id }),
                Some("secret-key"),
            ))
            .await
            .expect("run response"),
    )
    .await;
    let build_id = run_payload["data"]["run_job"]["id"]
        .as_str()
        .expect("build id")
        .to_string();

    let claim_payload = read_json(
        app.clone()
            .oneshot(graphql_request(
                r#"
                mutation Claim($workerId: String!) {
                  worker_claim_build(workerId: $workerId) { id }
                }
                "#,
                json!({ "workerId": "worker-a" }),
                Some("secret-key"),
            ))
            .await
            .expect("claim response"),
    )
    .await;
    assert_eq!(claim_payload["data"]["worker_claim_build"]["id"], build_id);

    let failed_payload = read_json(
        app.clone()
            .oneshot(graphql_request(
                r#"
                mutation Complete($workerId: String!, $buildId: ID!, $status: GqlWorkerBuildStatus!) {
                  worker_complete_build(workerId: $workerId, buildId: $buildId, status: $status) { id status }
                }
                "#,
                json!({
                    "workerId": "worker-a",
                    "buildId": build_id,
                    "status": "FAILED"
                }),
                Some("secret-key"),
            ))
            .await
            .expect("failed completion response"),
    )
    .await;
    assert_eq!(
        failed_payload["data"]["worker_complete_build"]["status"],
        "FAILED"
    );

    let dead_letter_payload = read_json(
        app.clone()
            .oneshot(graphql_request(
                "query DeadLetter { dead_letter_builds { id status } metrics { dead_letter_total } }",
                json!({}),
                None,
            ))
            .await
            .expect("dead letter query response"),
    )
    .await;

    assert_eq!(
        dead_letter_payload["data"]["metrics"]["dead_letter_total"],
        1
    );
    assert_eq!(
        dead_letter_payload["data"]["dead_letter_builds"][0]["id"],
        build_id
    );
    assert_eq!(
        dead_letter_payload["data"]["dead_letter_builds"][0]["status"],
        "FAILED"
    );
}
