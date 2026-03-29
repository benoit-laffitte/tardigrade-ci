use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use tower::ServiceExt;

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

#[tokio::test]
async fn graphql_dashboard_snapshot_returns_collections() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

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
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

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
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

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
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

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
