use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tardigrade_scheduler::{adapters::InMemoryScheduler, ports::Scheduler};
use tardigrade_storage::{adapters::InMemoryStorage, ports::Storage};
use tower::ServiceExt;

/// Builds one API router with explicit in-memory port implementations.
fn build_test_router() -> axum::Router {
    let storage: Arc<dyn Storage + Send + Sync> = Arc::new(InMemoryStorage::default());
    let scheduler: Arc<dyn Scheduler + Send + Sync> = Arc::new(InMemoryScheduler::default());
    let state = tardigrade_api::ApiState::with_components("tardigrade-ci-test", storage, scheduler);
    tardigrade_api::build_router(state)
}

fn graphql_request(query: &str, variables: Value) -> Request<Body> {
    let body = json!({
        "query": query,
        "variables": variables,
    });

    Request::builder()
        .method("POST")
        .uri("/graphql")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("valid graphql request")
}

async fn read_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&body).expect("valid json")
}

/// Verifies that legacy REST endpoints are no longer exposed by the API router.
#[tokio::test]
async fn graphql_router_does_not_expose_legacy_rest_endpoints() {
    let app = build_test_router();

    let health_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("health response");

    let jobs_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/jobs")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("jobs response");

    assert_eq!(health_response.status(), StatusCode::NOT_FOUND);
    assert_eq!(jobs_response.status(), StatusCode::NOT_FOUND);
}

/// Verifies API wiring accepts explicit storage/scheduler port trait objects.
#[tokio::test]
async fn graphql_router_accepts_port_trait_object_components() {
    let storage: Arc<dyn Storage + Send + Sync> = Arc::new(InMemoryStorage::default());
    let scheduler: Arc<dyn Scheduler + Send + Sync> = Arc::new(InMemoryScheduler::default());
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::with_components(
        "tardigrade-ci-test",
        storage,
        scheduler,
    ));

    let response = app
        .oneshot(graphql_request(
            r#"
            query Ready {
              ready { status }
            }
            "#,
            json!({}),
        ))
        .await
        .expect("graphql response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;
    assert!(payload.get("errors").is_none(), "graphql errors: {payload}");
    assert_eq!(payload["data"]["ready"]["status"], "ready");
}

#[tokio::test]
async fn graphql_dashboard_snapshot_returns_collections() {
    let app = build_test_router();

    let response = app
        .oneshot(graphql_request(
            r#"
            query Snapshot {
              dashboard_snapshot {
                jobs { id }
                builds { id }
                workers { id }
                metrics { reclaimed_total retry_requeued_total ownership_conflicts_total dead_letter_total }
                dead_letter_builds { id }
              }
            }
            "#,
            json!({}),
        ))
        .await
        .expect("graphql response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;

    assert!(payload.get("errors").is_none(), "graphql errors: {payload}");

    let snapshot = &payload["data"]["dashboard_snapshot"];
    assert!(snapshot["jobs"].is_array());
    assert!(snapshot["builds"].is_array());
    assert!(snapshot["workers"].is_array());
    assert!(snapshot["metrics"].is_object());
    assert!(snapshot["dead_letter_builds"].is_array());
}

#[tokio::test]
async fn graphql_create_and_run_job_flow_works() {
    let app = build_test_router();

    let create_response = app
        .clone()
        .oneshot(graphql_request(
            r#"
            mutation Create($input: GqlCreateJobInput!) {
              create_job(input: $input) {
                id
                name
              }
            }
            "#,
            json!({
                "input": {
                    "name": "build-graphql",
                    "repository_url": "https://example.com/repo.git",
                    "pipeline_path": "pipeline.yml"
                }
            }),
        ))
        .await
        .expect("create job response");

    assert_eq!(create_response.status(), StatusCode::OK);
    let create_payload = read_json(create_response).await;
    assert!(
        create_payload.get("errors").is_none(),
        "graphql errors: {create_payload}"
    );

    let job_id = create_payload["data"]["create_job"]["id"]
        .as_str()
        .expect("job id string")
        .to_string();

    let run_response = app
        .oneshot(graphql_request(
            r#"
            mutation Run($jobId: ID!) {
                            run_job(jobId: $jobId) {
                id
                status
              }
            }
            "#,
            json!({ "jobId": job_id }),
        ))
        .await
        .expect("run job response");

    assert_eq!(run_response.status(), StatusCode::OK);
    let run_payload = read_json(run_response).await;
    assert!(
        run_payload.get("errors").is_none(),
        "graphql errors: {run_payload}"
    );

    let status = run_payload["data"]["run_job"]["status"]
        .as_str()
        .expect("status string");
    assert!(status == "PENDING" || status == "SUCCESS" || status == "RUNNING");
}

#[tokio::test]
async fn graphql_create_job_with_invalid_pipeline_yaml_returns_error() {
    let app = build_test_router();

    let response = app
        .oneshot(graphql_request(
            r#"
            mutation Create($input: GqlCreateJobInput!) {
              create_job(input: $input) {
                id
              }
            }
            "#,
            json!({
                "input": {
                    "name": "build-graphql-invalid",
                    "repository_url": "https://example.com/repo.git",
                    "pipeline_path": "pipeline.yml",
                    "pipeline_yaml": "version: [1\nstages: []"
                }
            }),
        ))
        .await
        .expect("create job response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;

    let errors = payload["errors"].as_array().expect("graphql errors array");
    assert!(!errors.is_empty());
    let message = errors[0]["message"].as_str().expect("error message");
    assert!(message.contains("invalid pipeline YAML"));
    assert_eq!(errors[0]["extensions"]["code"], "invalid_pipeline");
}

#[tokio::test]
async fn graphql_create_job_with_structurally_invalid_pipeline_yaml_returns_details() {
    let app = build_test_router();

    let response = app
        .oneshot(graphql_request(
            r#"
            mutation Create($input: GqlCreateJobInput!) {
              create_job(input: $input) {
                id
              }
            }
            "#,
            json!({
                "input": {
                    "name": "build-graphql-invalid-structure",
                    "repository_url": "https://example.com/repo.git",
                    "pipeline_path": "pipeline.yml",
                    "pipeline_yaml": "version: 2\nstages:\n  - name: \"\"\n    steps:\n      - name: \"\"\n        image: \"\"\n        command: []"
                }
            }),
        ))
        .await
        .expect("create job response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;

    let errors = payload["errors"].as_array().expect("graphql errors array");
    assert!(!errors.is_empty());
    assert_eq!(errors[0]["extensions"]["code"], "invalid_pipeline");

    let details = errors[0]["extensions"]["details"]
        .as_array()
        .expect("validation details array");
    assert!(!details.is_empty());
    assert!(
        details
            .iter()
            .any(|issue| issue["field"].as_str() == Some("version"))
    );
}

#[tokio::test]
/// Ensures blank inline pipeline content returns a bad request style GraphQL error.
async fn graphql_create_job_with_blank_pipeline_yaml_returns_bad_request_error() {
    let app = build_test_router();

    let response = app
        .oneshot(graphql_request(
            r#"
            mutation Create($input: GqlCreateJobInput!) {
              create_job(input: $input) {
                id
              }
            }
            "#,
            json!({
                "input": {
                    "name": "build-graphql-blank-yaml",
                    "repository_url": "https://example.com/repo.git",
                    "pipeline_path": "pipeline.yml",
                    "pipeline_yaml": "   \n\t"
                }
            }),
        ))
        .await
        .expect("create job response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;

    let errors = payload["errors"].as_array().expect("graphql errors array");
    assert!(!errors.is_empty());
    let message = errors[0]["message"].as_str().expect("error message");
    assert!(message.contains("status 400"));
}

#[tokio::test]
/// Ensures retry policy constraint failures are exposed in GraphQL error details.
async fn graphql_create_job_with_invalid_retry_policy_returns_retry_field_details() {
    let app = build_test_router();

    let response = app
        .oneshot(graphql_request(
            r#"
            mutation Create($input: GqlCreateJobInput!) {
              create_job(input: $input) {
                id
              }
            }
            "#,
            json!({
                "input": {
                    "name": "build-graphql-invalid-retry",
                    "repository_url": "https://example.com/repo.git",
                    "pipeline_path": "pipeline.yml",
                    "pipeline_yaml": "version: 1\nstages:\n  - name: build\n    steps:\n      - name: cargo-build\n        image: \"rust:1.94\"\n        command:\n          - cargo\n          - build\n        retry:\n          max_attempts: 0\n          backoff_ms: 0"
                }
            }),
        ))
        .await
        .expect("create job response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;

    let errors = payload["errors"].as_array().expect("graphql errors array");
    assert!(!errors.is_empty());
    assert_eq!(errors[0]["extensions"]["code"], "invalid_pipeline");

    let details = errors[0]["extensions"]["details"]
        .as_array()
        .expect("validation details array");
    assert!(
        details.iter().any(|issue| {
            issue["field"].as_str() == Some("stages[0].steps[0].retry.max_attempts")
        })
    );
    assert!(
        details.iter().any(|issue| {
            issue["field"].as_str() == Some("stages[0].steps[0].retry.backoff_ms")
        })
    );
}

/// Ensures plugin lifecycle administration remains available through GraphQL only.
#[tokio::test]
async fn graphql_plugin_lifecycle_flow_works() {
    let app = build_test_router();

    let response = app
        .oneshot(graphql_request(
            r#"
            mutation PluginFlow($name: String!) {
              load_plugin(name: $name) { name state capabilities }
              init_plugin(name: $name) { name state }
              execute_plugin(name: $name) { name state }
              unload_plugin(name: $name) { name state }
            }
            "#,
            json!({ "name": "net-diagnostics" }),
        ))
        .await
        .expect("plugin flow response");

    assert_eq!(response.status(), StatusCode::OK);
    let payload = read_json(response).await;
    assert!(payload.get("errors").is_none(), "graphql errors: {payload}");

    assert_eq!(payload["data"]["load_plugin"]["state"], "Loaded");
    assert_eq!(payload["data"]["init_plugin"]["state"], "Initialized");
    assert_eq!(payload["data"]["execute_plugin"]["name"], "net-diagnostics");
    assert_eq!(payload["data"]["unload_plugin"]["state"], "Unloaded");
}

/// Ensures SCM configuration, webhook ingestion, and diagnostics remain reachable through GraphQL.
#[tokio::test]
async fn graphql_scm_flow_works_without_rest_endpoints() {
    let app = build_test_router();

    let create_job = app
        .clone()
        .oneshot(graphql_request(
            r#"
            mutation Create($input: GqlCreateJobInput!) {
              create_job(input: $input) { id }
            }
            "#,
            json!({
                "input": {
                    "name": "scm-graphql",
                    "repository_url": "https://example.com/repo.git",
                    "pipeline_path": "pipeline.yml"
                }
            }),
        ))
        .await
        .expect("create job response");
    assert_eq!(create_job.status(), StatusCode::OK);

    let secret = "topsecret";
    let body = r#"{"after":"abc123","ref":"refs/heads/main"}"#;
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let signature = {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("valid hmac key");
        mac.update(body.as_bytes());
        format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
    };

    let config_response = app
        .clone()
        .oneshot(graphql_request(
            r#"
            mutation Configure($input: GqlUpsertWebhookSecurityConfigInput!, $poll: GqlUpsertScmPollingConfigInput!) {
              upsert_webhook_security_config(input: $input)
              upsert_scm_polling_config(input: $poll)
            }
            "#,
            json!({
                "input": {
                    "repository_url": "https://example.com/repo.git",
                    "provider": "GITHUB",
                    "secret": secret,
                    "allowed_ips": []
                },
                "poll": {
                    "repository_url": "https://example.com/repo.git",
                    "provider": "GITHUB",
                    "enabled": true,
                    "interval_secs": 30,
                    "branches": ["main"]
                }
            }),
        ))
        .await
        .expect("config response");
    assert_eq!(config_response.status(), StatusCode::OK);
    let config_payload = read_json(config_response).await;
    assert!(
        config_payload.get("errors").is_none(),
        "graphql errors: {config_payload}"
    );

    let webhook_response = app
        .clone()
        .oneshot(graphql_request(
            r#"
            mutation Ingest($headers: [GqlWebhookHeaderInput!]!, $body: String!) {
              ingest_scm_webhook(headers: $headers, body: $body)
            }
            "#,
            json!({
                "headers": [
                    { "name": "x-scm-provider", "value": "github" },
                    { "name": "x-scm-repository", "value": "https://example.com/repo.git" },
                    { "name": "x-scm-timestamp", "value": timestamp },
                    { "name": "x-hub-signature-256", "value": signature },
                    { "name": "x-github-event", "value": "push" },
                    { "name": "x-github-delivery", "value": "delivery-1" }
                ],
                "body": body
            }),
        ))
        .await
        .expect("webhook response");
    assert_eq!(webhook_response.status(), StatusCode::OK);
    let webhook_payload = read_json(webhook_response).await;
    assert!(
        webhook_payload.get("errors").is_none(),
        "graphql errors: {webhook_payload}"
    );
    assert_eq!(webhook_payload["data"]["ingest_scm_webhook"], true);

    let builds_response = app
        .oneshot(graphql_request(
            r#"
            query BuildsAndDiagnostics {
              builds { id }
              scm_webhook_rejections { reason_code }
              metrics { scm_webhook_received_total scm_webhook_accepted_total scm_trigger_enqueued_builds_total }
            }
            "#,
            json!({}),
        ))
        .await
        .expect("builds response");
    assert_eq!(builds_response.status(), StatusCode::OK);
    let builds_payload = read_json(builds_response).await;
    assert!(
        builds_payload.get("errors").is_none(),
        "graphql errors: {builds_payload}"
    );
    assert_eq!(
        builds_payload["data"]["builds"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(
        builds_payload["data"]["scm_webhook_rejections"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        builds_payload["data"]["metrics"]["scm_webhook_received_total"],
        1
    );
    assert_eq!(
        builds_payload["data"]["metrics"]["scm_webhook_accepted_total"],
        1
    );
    assert_eq!(
        builds_payload["data"]["metrics"]["scm_trigger_enqueued_builds_total"],
        1
    );
}

/// Ensures invalid webhook signatures still expose the expected GraphQL code and rejection metrics.
#[tokio::test]
async fn graphql_scm_webhook_invalid_signature_reports_auth_rejection() {
    let app = build_test_router();

    let secret = "topsecret";
    let body = r#"{"after":"abc123","ref":"refs/heads/main"}"#;
    let timestamp = chrono::Utc::now().timestamp().to_string();

    let config_response = app
        .clone()
        .oneshot(graphql_request(
            r#"
            mutation Configure($input: GqlUpsertWebhookSecurityConfigInput!) {
              upsert_webhook_security_config(input: $input)
            }
            "#,
            json!({
                "input": {
                    "repository_url": "https://example.com/repo.git",
                    "provider": "GITHUB",
                    "secret": secret,
                    "allowed_ips": []
                }
            }),
        ))
        .await
        .expect("config response");
    assert_eq!(config_response.status(), StatusCode::OK);

    let webhook_response = app
        .clone()
        .oneshot(graphql_request(
            r#"
            mutation Ingest($headers: [GqlWebhookHeaderInput!]!, $body: String!) {
              ingest_scm_webhook(headers: $headers, body: $body)
            }
            "#,
            json!({
                "headers": [
                    { "name": "x-scm-provider", "value": "github" },
                    { "name": "x-scm-repository", "value": "https://example.com/repo.git" },
                    { "name": "x-scm-timestamp", "value": timestamp },
                    { "name": "x-hub-signature-256", "value": "sha256=deadbeef" },
                    { "name": "x-github-event", "value": "push" },
                    { "name": "x-github-delivery", "value": "delivery-2" }
                ],
                "body": body
            }),
        ))
        .await
        .expect("webhook response");
    assert_eq!(webhook_response.status(), StatusCode::OK);

    let webhook_payload = read_json(webhook_response).await;
    let errors = webhook_payload["errors"]
        .as_array()
        .expect("graphql errors array");
    assert!(!errors.is_empty());
    assert_eq!(errors[0]["extensions"]["code"], "invalid_webhook_signature");

    let diagnostics_response = app
        .oneshot(graphql_request(
            r#"
            query Diagnostics {
              scm_webhook_rejections { reason_code }
              metrics { scm_webhook_received_total scm_webhook_rejected_total scm_webhook_accepted_total }
            }
            "#,
            json!({}),
        ))
        .await
        .expect("diagnostics response");
    assert_eq!(diagnostics_response.status(), StatusCode::OK);

    let diagnostics_payload = read_json(diagnostics_response).await;
    assert!(
        diagnostics_payload.get("errors").is_none(),
        "graphql errors: {diagnostics_payload}"
    );

    assert_eq!(
        diagnostics_payload["data"]["scm_webhook_rejections"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        diagnostics_payload["data"]["scm_webhook_rejections"][0]["reason_code"],
        "invalid_webhook_signature"
    );
    assert_eq!(
        diagnostics_payload["data"]["metrics"]["scm_webhook_received_total"],
        1
    );
    assert_eq!(
        diagnostics_payload["data"]["metrics"]["scm_webhook_rejected_total"],
        1
    );
    assert_eq!(
        diagnostics_payload["data"]["metrics"]["scm_webhook_accepted_total"],
        0
    );
}
