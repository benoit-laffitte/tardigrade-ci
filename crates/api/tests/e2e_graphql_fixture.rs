use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tardigrade_scheduler::{adapters::InMemoryScheduler, ports::Scheduler};
use tardigrade_storage::{adapters::InMemoryStorage, ports::Storage};
use tower::ServiceExt;

/// Deterministic GraphQL fixture exposing one in-memory server+worker lifecycle harness.
struct GraphqlE2eFixture {
    /// Test router wired with in-memory ports.
    app: axum::Router,
    /// Stable worker identity used across claim/complete calls.
    worker_id: String,
}

/// Minimal build projection returned by fixture list helpers.
struct BuildSnapshot {
    /// Build identifier.
    id: String,
    /// Build lifecycle status.
    status: String,
}

impl GraphqlE2eFixture {
    /// Builds a new deterministic fixture with explicit in-memory wiring.
    fn new(worker_id: &str) -> Self {
        let storage: Arc<dyn Storage + Send + Sync> = Arc::new(InMemoryStorage::default());
        let scheduler: Arc<dyn Scheduler + Send + Sync> = Arc::new(InMemoryScheduler::default());
        let state =
            tardigrade_api::ApiState::with_components("tardigrade-ci-e2e", storage, scheduler);
        let app = tardigrade_api::build_router(state);

        Self {
            app,
            worker_id: worker_id.to_string(),
        }
    }

    /// Sends one GraphQL request and returns the decoded JSON payload.
    async fn graphql(&self, query: &str, variables: Value) -> Value {
        let body = json!({
            "query": query,
            "variables": variables,
        });

        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/graphql")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("valid GraphQL request"),
            )
            .await
            .expect("graphql response");

        assert_eq!(response.status(), StatusCode::OK);

        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read graphql response body");
        serde_json::from_slice(&bytes).expect("valid graphql json payload")
    }

    /// Queries server readiness status.
    async fn ready_status(&self) -> String {
        let payload = self
            .graphql(
                r#"
                query Ready {
                  ready { status }
                }
                "#,
                json!({}),
            )
            .await;

        assert!(payload.get("errors").is_none(), "graphql errors: {payload}");
        payload["data"]["ready"]["status"]
            .as_str()
            .expect("ready status string")
            .to_string()
    }

    /// Queries service health status.
    async fn health_status(&self) -> String {
        let payload = self
            .graphql(
                r#"
                query Health {
                  health { status }
                }
                "#,
                json!({}),
            )
            .await;

        assert!(payload.get("errors").is_none(), "graphql errors: {payload}");
        payload["data"]["health"]["status"]
            .as_str()
            .expect("health status string")
            .to_string()
    }

    /// Creates one job and returns its identifier.
    async fn create_job(&self, name: &str) -> String {
        let payload = self
            .graphql(
                r#"
                mutation Create($input: GqlCreateJobInput!) {
                  create_job(input: $input) { id }
                }
                "#,
                json!({
                    "input": {
                        "name": name,
                        "repository_url": "https://example.com/repo.git",
                        "pipeline_path": "pipeline.yml"
                    }
                }),
            )
            .await;

        assert!(payload.get("errors").is_none(), "graphql errors: {payload}");
        payload["data"]["create_job"]["id"]
            .as_str()
            .expect("job id string")
            .to_string()
    }

    /// Starts one build for the provided job id and returns the build identifier.
    async fn run_job(&self, job_id: &str) -> String {
        let payload = self
            .graphql(
                r#"
                mutation Run($jobId: ID!) {
                  run_job(jobId: $jobId) { id }
                }
                "#,
                json!({ "jobId": job_id }),
            )
            .await;

        assert!(payload.get("errors").is_none(), "graphql errors: {payload}");
        payload["data"]["run_job"]["id"]
            .as_str()
            .expect("build id string")
            .to_string()
    }

    /// Claims one build for the fixture worker and returns the optional claimed build id.
    async fn claim_build(&self) -> Option<String> {
        let payload = self
            .graphql(
                r#"
                mutation Claim($workerId: String!) {
                  worker_claim_build(workerId: $workerId) { id status }
                }
                "#,
                json!({ "workerId": self.worker_id }),
            )
            .await;

        assert!(payload.get("errors").is_none(), "graphql errors: {payload}");

        if payload["data"]["worker_claim_build"].is_null() {
            return None;
        }

        assert_eq!(payload["data"]["worker_claim_build"]["status"], "RUNNING");
        Some(
            payload["data"]["worker_claim_build"]["id"]
                .as_str()
                .expect("claimed build id string")
                .to_string(),
        )
    }

    /// Completes one claimed build with SUCCESS status.
    async fn complete_build_success(&self, build_id: &str) -> Value {
        let payload = self
            .graphql(
                r#"
                mutation Complete($workerId: String!, $buildId: ID!, $status: GqlWorkerBuildStatus!) {
                  worker_complete_build(workerId: $workerId, buildId: $buildId, status: $status) { id status }
                }
                "#,
                json!({
                    "workerId": self.worker_id,
                    "buildId": build_id,
                    "status": "SUCCESS"
                }),
            )
            .await;

        assert!(payload.get("errors").is_none(), "graphql errors: {payload}");
        payload
    }

    /// Lists build snapshots currently visible through GraphQL.
    async fn list_builds(&self) -> Vec<BuildSnapshot> {
        let payload = self
            .graphql(
                r#"
                query Builds {
                  builds { id status }
                }
                "#,
                json!({}),
            )
            .await;

        assert!(payload.get("errors").is_none(), "graphql errors: {payload}");
        payload["data"]["builds"]
            .as_array()
            .expect("build list array")
            .iter()
            .map(|entry| BuildSnapshot {
                id: entry["id"].as_str().expect("build id string").to_string(),
                status: entry["status"]
                    .as_str()
                    .expect("build status string")
                    .to_string(),
            })
            .collect()
    }
}

/// Verifies deterministic GraphQL happy path for health/create/run/claim/complete/list lifecycle.
#[tokio::test]
async fn graphql_e2e_happy_path_covers_health_create_run_claim_complete_and_list_builds() {
    let fixture = GraphqlE2eFixture::new("worker-e2e");

    let health = fixture.health_status().await;
    assert_eq!(health, "ok");

    let ready = fixture.ready_status().await;
    assert_eq!(ready, "ready");

    let job_id = fixture.create_job("e2e-fixture-job").await;
    let build_id = fixture.run_job(&job_id).await;

    let claimed_build_id = fixture
        .claim_build()
        .await
        .expect("one build should be claimable");
    assert_eq!(claimed_build_id, build_id);

    let complete_payload = fixture.complete_build_success(&build_id).await;
    assert_eq!(
        complete_payload["data"]["worker_complete_build"]["status"],
        "SUCCESS"
    );

    let builds = fixture.list_builds().await;
    assert_eq!(builds.len(), 1);
    assert_eq!(builds[0].id, build_id);
    assert_eq!(builds[0].status, "SUCCESS");

    let second_claim = fixture.claim_build().await;
    assert!(
        second_claim.is_none(),
        "queue should be empty after completion"
    );
}
