use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::sync::Arc;
use tardigrade_api::{
    CancelBuildResponse, ClaimBuildResponse, CompleteBuildRequest, CompleteBuildResponse,
    CreateJobResponse, DeadLetterBuildsResponse, ListBuildsResponse, ListJobsResponse,
    ListWorkersResponse, RunJobResponse, RuntimeMetricsResponse, ServiceSettings,
    WorkerBuildStatus,
};
use tardigrade_core::JobStatus;
use tardigrade_scheduler::InMemoryScheduler;
use tardigrade_storage::InMemoryStorage;
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Serialize)]
struct CreateJobRequestBody {
    name: String,
    repository_url: String,
    pipeline_path: String,
    pipeline_yaml: Option<String>,
}

#[tokio::test]
async fn create_and_list_jobs() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let create_body = CreateJobRequestBody {
        name: "build-backend".to_string(),
        repository_url: "https://example.com/repo.git".to_string(),
        pipeline_path: "pipeline.yml".to_string(),
        pipeline_yaml: None,
    };

    let create_response = app
        .clone()
        .oneshot(json_request("/jobs", &create_body))
        .await
        .expect("create response");

    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created: CreateJobResponse = read_json_body(create_response).await;

    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/jobs")
                .body(Body::empty())
                .expect("valid list request"),
        )
        .await
        .expect("list response");

    assert_eq!(list_response.status(), StatusCode::OK);
    let listed: ListJobsResponse = read_json_body(list_response).await;

    assert_eq!(listed.jobs.len(), 1);
    assert_eq!(listed.jobs[0].id, created.job.id);
    assert_eq!(listed.jobs[0].name, "build-backend");
}

#[tokio::test]
async fn run_and_cancel_build() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let create_body = CreateJobRequestBody {
        name: "build-api".to_string(),
        repository_url: "https://example.com/api.git".to_string(),
        pipeline_path: "pipelines/api.yml".to_string(),
        pipeline_yaml: None,
    };

    let create_response = app
        .clone()
        .oneshot(json_request("/jobs", &create_body))
        .await
        .expect("create response");

    let created: CreateJobResponse = read_json_body(create_response).await;

    let run_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/jobs/{}/run", created.job.id))
                .body(Body::empty())
                .expect("valid run request"),
        )
        .await
        .expect("run response");

    assert_eq!(run_response.status(), StatusCode::CREATED);
    let run_payload: RunJobResponse = read_json_body(run_response).await;
    assert_eq!(run_payload.build.status, JobStatus::Pending);

    let cancel_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/builds/{}/cancel", run_payload.build.id))
                .body(Body::empty())
                .expect("valid cancel request"),
        )
        .await
        .expect("cancel response");

    assert_eq!(cancel_response.status(), StatusCode::OK);
    let canceled: CancelBuildResponse = read_json_body(cancel_response).await;
    assert_eq!(canceled.build.status, JobStatus::Canceled);
}

#[tokio::test]
async fn create_job_with_empty_name_returns_bad_request() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let invalid_body = CreateJobRequestBody {
        name: "   ".to_string(),
        repository_url: "https://example.com/repo.git".to_string(),
        pipeline_path: "pipeline.yml".to_string(),
        pipeline_yaml: None,
    };

    let response = app
        .oneshot(json_request("/jobs", &invalid_body))
        .await
        .expect("create response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn run_unknown_job_returns_not_found() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));
    let unknown_id = Uuid::new_v4();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/jobs/{unknown_id}/run"))
                .body(Body::empty())
                .expect("valid run request"),
        )
        .await
        .expect("run response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_job_with_invalid_pipeline_yaml_returns_unprocessable_entity() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let invalid_body = CreateJobRequestBody {
        name: "build-invalid-yaml".to_string(),
        repository_url: "https://example.com/repo.git".to_string(),
        pipeline_path: "pipeline.yml".to_string(),
        pipeline_yaml: Some("version: [1\nstages: []".to_string()),
    };

    let response = app
        .oneshot(json_request("/jobs", &invalid_body))
        .await
        .expect("create response");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let payload: Value = read_json_body(response).await;
    assert_eq!(payload["code"], "invalid_pipeline");
    assert!(
        payload["message"]
            .as_str()
            .expect("error message")
            .contains("invalid pipeline YAML")
    );
    assert!(payload["details"].is_null());
}

#[tokio::test]
async fn create_job_with_valid_pipeline_yaml_returns_created() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let valid_body = CreateJobRequestBody {
        name: "build-valid-yaml".to_string(),
        repository_url: "https://example.com/repo.git".to_string(),
        pipeline_path: "pipeline.yml".to_string(),
        pipeline_yaml: Some(
            "version: 1\nstages:\n  - name: build\n    steps:\n      - name: cargo-build\n        image: \"rust:1.94\"\n        command:\n          - cargo\n          - build\n"
                .to_string(),
        ),
    };

    let response = app
        .oneshot(json_request("/jobs", &valid_body))
        .await
        .expect("create response");

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn create_job_with_structurally_invalid_pipeline_yaml_returns_details() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let invalid_body = CreateJobRequestBody {
        name: "build-invalid-structure".to_string(),
        repository_url: "https://example.com/repo.git".to_string(),
        pipeline_path: "pipeline.yml".to_string(),
        pipeline_yaml: Some(
            "version: 2\nstages:\n  - name: \"\"\n    steps:\n      - name: \"\"\n        image: \"\"\n        command: []\n"
                .to_string(),
        ),
    };

    let response = app
        .oneshot(json_request("/jobs", &invalid_body))
        .await
        .expect("create response");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let payload: Value = read_json_body(response).await;
    assert_eq!(payload["code"], "invalid_pipeline");
    assert!(
        payload["message"]
            .as_str()
            .expect("error message")
            .contains("pipeline validation failed")
    );
    let details = payload["details"].as_array().expect("details array");
    assert!(!details.is_empty());
    assert!(
        details
            .iter()
            .any(|issue| issue["field"].as_str() == Some("version"))
    );
}

#[tokio::test]
/// Ensures blank inline pipeline content is rejected as a malformed request body.
async fn create_job_with_blank_pipeline_yaml_returns_bad_request() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let invalid_body = CreateJobRequestBody {
        name: "build-blank-yaml".to_string(),
        repository_url: "https://example.com/repo.git".to_string(),
        pipeline_path: "pipeline.yml".to_string(),
        pipeline_yaml: Some("   \n\t".to_string()),
    };

    let response = app
        .oneshot(json_request("/jobs", &invalid_body))
        .await
        .expect("create response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
/// Ensures retry policy constraints are surfaced in invalid-pipeline details.
async fn create_job_with_invalid_retry_policy_returns_retry_field_details() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let invalid_body = CreateJobRequestBody {
        name: "build-invalid-retry".to_string(),
        repository_url: "https://example.com/repo.git".to_string(),
        pipeline_path: "pipeline.yml".to_string(),
        pipeline_yaml: Some(
            "version: 1\nstages:\n  - name: build\n    steps:\n      - name: cargo-build\n        image: \"rust:1.94\"\n        command:\n          - cargo\n          - build\n        retry:\n          max_attempts: 0\n          backoff_ms: 0\n"
                .to_string(),
        ),
    };

    let response = app
        .oneshot(json_request("/jobs", &invalid_body))
        .await
        .expect("create response");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let payload: Value = read_json_body(response).await;
    let details = payload["details"].as_array().expect("details array");

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

#[tokio::test]
async fn cancel_unknown_build_returns_not_found() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));
    let unknown_id = Uuid::new_v4();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/builds/{unknown_id}/cancel"))
                .body(Body::empty())
                .expect("valid cancel request"),
        )
        .await
        .expect("cancel response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn run_job_is_eventually_marked_success() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let create_body = CreateJobRequestBody {
        name: "build-worker".to_string(),
        repository_url: "https://example.com/worker.git".to_string(),
        pipeline_path: "pipelines/worker.yml".to_string(),
        pipeline_yaml: None,
    };

    let create_response = app
        .clone()
        .oneshot(json_request("/jobs", &create_body))
        .await
        .expect("create response");
    let created: CreateJobResponse = read_json_body(create_response).await;

    let run_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/jobs/{}/run", created.job.id))
                .body(Body::empty())
                .expect("valid run request"),
        )
        .await
        .expect("run response");
    assert_eq!(run_response.status(), StatusCode::CREATED);
    let run_payload: RunJobResponse = read_json_body(run_response).await;

    let mut final_status = JobStatus::Pending;
    for _ in 0..20 {
        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/builds")
                    .body(Body::empty())
                    .expect("valid list builds request"),
            )
            .await
            .expect("list builds response");

        let listed: ListBuildsResponse = read_json_body(list_response).await;
        let build = listed
            .builds
            .iter()
            .find(|b| b.id == run_payload.build.id)
            .expect("created build should exist");
        final_status = build.status.clone();

        if final_status == JobStatus::Success {
            break;
        }

        tokio::time::sleep(std::time::Duration::from_millis(15)).await;
    }

    assert_eq!(final_status, JobStatus::Success);
}

#[tokio::test]
/// Ensures Rust/Python/Java pipeline templates can be created and executed end-to-end.
async fn run_smoke_matrix_templates_are_eventually_successful() {
    let app = tardigrade_api::build_router(tardigrade_api::ApiState::new("tardigrade-ci-test"));

    let templates = vec![
        (
            "rust",
            "version: 1\nstages:\n  - name: build\n    steps:\n      - name: cargo-build\n        image: \"rust:1.94\"\n        command:\n          - cargo\n          - build\n          - --workspace\n",
        ),
        (
            "python",
            "version: 1\nstages:\n  - name: verify\n    steps:\n      - name: pytest\n        image: \"python:3.12\"\n        command:\n          - pytest\n          - -q\n",
        ),
        (
            "java",
            "version: 1\nstages:\n  - name: verify\n    steps:\n      - name: maven-test\n        image: \"maven:3.9-eclipse-temurin-21\"\n        command:\n          - mvn\n          - -B\n          - test\n",
        ),
    ];

    for (stack_name, pipeline_yaml) in templates {
        let create_body = CreateJobRequestBody {
            name: format!("smoke-{stack_name}"),
            repository_url: format!("https://example.com/{stack_name}.git"),
            pipeline_path: "pipeline.yml".to_string(),
            pipeline_yaml: Some(pipeline_yaml.to_string()),
        };

        let create_response = app
            .clone()
            .oneshot(json_request("/jobs", &create_body))
            .await
            .expect("create response");
        assert_eq!(create_response.status(), StatusCode::CREATED);
        let created: CreateJobResponse = read_json_body(create_response).await;

        let run_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/jobs/{}/run", created.job.id))
                    .body(Body::empty())
                    .expect("valid run request"),
            )
            .await
            .expect("run response");
        assert_eq!(run_response.status(), StatusCode::CREATED);
        let run_payload: RunJobResponse = read_json_body(run_response).await;

        let mut final_status = JobStatus::Pending;
        for _ in 0..20 {
            let list_response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/builds")
                        .body(Body::empty())
                        .expect("valid list builds request"),
                )
                .await
                .expect("list builds response");

            let listed: ListBuildsResponse = read_json_body(list_response).await;
            let build = listed
                .builds
                .iter()
                .find(|b| b.id == run_payload.build.id)
                .expect("created build should exist");
            final_status = build.status.clone();

            if final_status == JobStatus::Success {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(15)).await;
        }

        assert_eq!(
            final_status,
            JobStatus::Success,
            "smoke matrix stack should complete successfully: {stack_name}"
        );
    }
}

#[tokio::test]
async fn external_worker_can_claim_and_complete_build() {
    let state = tardigrade_api::ApiState::with_components_and_mode(
        "tardigrade-ci-test",
        Arc::new(InMemoryStorage::default()),
        Arc::new(InMemoryScheduler::default()),
        false,
    );
    let app = tardigrade_api::build_router(state);

    let create_body = CreateJobRequestBody {
        name: "build-external-worker".to_string(),
        repository_url: "https://example.com/ext.git".to_string(),
        pipeline_path: "pipelines/ext.yml".to_string(),
        pipeline_yaml: None,
    };

    let create_response = app
        .clone()
        .oneshot(json_request("/jobs", &create_body))
        .await
        .expect("create response");
    let created: CreateJobResponse = read_json_body(create_response).await;

    let run_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/jobs/{}/run", created.job.id))
                .body(Body::empty())
                .expect("valid run request"),
        )
        .await
        .expect("run response");
    assert_eq!(run_response.status(), StatusCode::CREATED);
    let run_payload: RunJobResponse = read_json_body(run_response).await;

    let claim_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/workers/worker-a/claim")
                .body(Body::empty())
                .expect("valid claim request"),
        )
        .await
        .expect("claim response");
    assert_eq!(claim_response.status(), StatusCode::OK);
    let claimed: ClaimBuildResponse = read_json_body(claim_response).await;
    let claimed_build = claimed.build.expect("build should be claimed");
    assert_eq!(claimed_build.id, run_payload.build.id);
    assert_eq!(claimed_build.status, JobStatus::Running);

    let workers_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/workers")
                .body(Body::empty())
                .expect("valid list workers request"),
        )
        .await
        .expect("list workers response");
    assert_eq!(workers_response.status(), StatusCode::OK);
    let workers_payload: ListWorkersResponse = read_json_body(workers_response).await;
    let worker = workers_payload
        .workers
        .iter()
        .find(|w| w.id == "worker-a")
        .expect("worker should be visible in list");
    assert_eq!(worker.status, "busy");
    assert_eq!(worker.active_builds, 1);

    let complete_body = CompleteBuildRequest {
        status: WorkerBuildStatus::Success,
        log_line: Some("External worker completed workload".to_string()),
    };

    let complete_response = app
        .clone()
        .oneshot(json_request(
            &format!("/workers/worker-a/builds/{}/complete", claimed_build.id),
            &complete_body,
        ))
        .await
        .expect("complete response");
    assert_eq!(complete_response.status(), StatusCode::OK);
    let completed: CompleteBuildResponse = read_json_body(complete_response).await;
    assert_eq!(completed.build.status, JobStatus::Success);

    let workers_response_after = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/workers")
                .body(Body::empty())
                .expect("valid list workers request"),
        )
        .await
        .expect("list workers response");
    let workers_payload_after: ListWorkersResponse = read_json_body(workers_response_after).await;
    let worker_after = workers_payload_after
        .workers
        .iter()
        .find(|w| w.id == "worker-a")
        .expect("worker should still be visible in list");
    assert_eq!(worker_after.status, "idle");
    assert_eq!(worker_after.active_builds, 0);
}

#[tokio::test]
async fn worker_cannot_complete_build_claimed_by_other_worker() {
    let state = tardigrade_api::ApiState::with_components_and_mode(
        "tardigrade-ci-test",
        Arc::new(InMemoryStorage::default()),
        Arc::new(InMemoryScheduler::default()),
        false,
    );
    let app = tardigrade_api::build_router(state);

    let create_body = CreateJobRequestBody {
        name: "build-ownership-check".to_string(),
        repository_url: "https://example.com/owner.git".to_string(),
        pipeline_path: "pipelines/owner.yml".to_string(),
        pipeline_yaml: None,
    };

    let create_response = app
        .clone()
        .oneshot(json_request("/jobs", &create_body))
        .await
        .expect("create response");
    let created: CreateJobResponse = read_json_body(create_response).await;

    let run_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/jobs/{}/run", created.job.id))
                .body(Body::empty())
                .expect("valid run request"),
        )
        .await
        .expect("run response");
    assert_eq!(run_response.status(), StatusCode::CREATED);
    let run_payload: RunJobResponse = read_json_body(run_response).await;

    let claim_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/workers/worker-a/claim")
                .body(Body::empty())
                .expect("valid claim request"),
        )
        .await
        .expect("claim response");
    assert_eq!(claim_response.status(), StatusCode::OK);

    let complete_body = CompleteBuildRequest {
        status: WorkerBuildStatus::Success,
        log_line: Some("Wrong worker tried to complete".to_string()),
    };

    let complete_response = app
        .clone()
        .oneshot(json_request(
            &format!("/workers/worker-b/builds/{}/complete", run_payload.build.id),
            &complete_body,
        ))
        .await
        .expect("complete response");
    assert_eq!(complete_response.status(), StatusCode::CONFLICT);

    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/builds")
                .body(Body::empty())
                .expect("valid list builds request"),
        )
        .await
        .expect("list builds response");
    let listed: ListBuildsResponse = read_json_body(list_response).await;
    let build = listed
        .builds
        .iter()
        .find(|b| b.id == run_payload.build.id)
        .expect("build should exist");
    assert_eq!(build.status, JobStatus::Running);
}

#[tokio::test]
async fn stale_claim_is_reclaimed_for_another_worker() {
    let state = tardigrade_api::ApiState::with_components_and_mode_and_settings(
        "tardigrade-ci-test",
        Arc::new(InMemoryStorage::default()),
        Arc::new(InMemoryScheduler::default()),
        false,
        ServiceSettings {
            worker_lease_timeout_secs: 0,
            max_retries: 2,
            retry_backoff_ms: 1000,
        },
    );
    let app = tardigrade_api::build_router(state);

    let create_body = CreateJobRequestBody {
        name: "build-reclaim-check".to_string(),
        repository_url: "https://example.com/reclaim.git".to_string(),
        pipeline_path: "pipelines/reclaim.yml".to_string(),
        pipeline_yaml: None,
    };

    let create_response = app
        .clone()
        .oneshot(json_request("/jobs", &create_body))
        .await
        .expect("create response");
    let created: CreateJobResponse = read_json_body(create_response).await;

    let run_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/jobs/{}/run", created.job.id))
                .body(Body::empty())
                .expect("valid run request"),
        )
        .await
        .expect("run response");
    let run_payload: RunJobResponse = read_json_body(run_response).await;

    let claim_a = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/workers/worker-a/claim")
                .body(Body::empty())
                .expect("valid claim request"),
        )
        .await
        .expect("claim response");
    assert_eq!(claim_a.status(), StatusCode::OK);

    let claim_b = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/workers/worker-b/claim")
                .body(Body::empty())
                .expect("valid claim request"),
        )
        .await
        .expect("claim response");
    assert_eq!(claim_b.status(), StatusCode::OK);
    let claimed_b: ClaimBuildResponse = read_json_body(claim_b).await;
    let build_b = claimed_b.build.expect("build should be reclaimed");
    assert_eq!(build_b.id, run_payload.build.id);
    assert_eq!(build_b.status, JobStatus::Running);

    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/builds")
                .body(Body::empty())
                .expect("valid list builds request"),
        )
        .await
        .expect("list builds response");
    let listed: ListBuildsResponse = read_json_body(list_response).await;
    let build = listed
        .builds
        .iter()
        .find(|b| b.id == run_payload.build.id)
        .expect("build should exist");
    assert!(
        build
            .logs
            .iter()
            .any(|line| line.contains("stale worker lease timeout"))
    );
}

#[tokio::test]
async fn failed_build_is_requeued_with_retry_and_exposed_in_metrics() {
    let state = tardigrade_api::ApiState::with_components_and_mode_and_settings(
        "tardigrade-ci-test",
        Arc::new(InMemoryStorage::default()),
        Arc::new(InMemoryScheduler::default()),
        false,
        ServiceSettings {
            worker_lease_timeout_secs: 30,
            max_retries: 1,
            retry_backoff_ms: 0,
        },
    );
    let app = tardigrade_api::build_router(state);

    let create_body = CreateJobRequestBody {
        name: "build-retry-check".to_string(),
        repository_url: "https://example.com/retry.git".to_string(),
        pipeline_path: "pipelines/retry.yml".to_string(),
        pipeline_yaml: None,
    };

    let create_response = app
        .clone()
        .oneshot(json_request("/jobs", &create_body))
        .await
        .expect("create response");
    let created: CreateJobResponse = read_json_body(create_response).await;

    let run_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/jobs/{}/run", created.job.id))
                .body(Body::empty())
                .expect("valid run request"),
        )
        .await
        .expect("run response");
    let run_payload: RunJobResponse = read_json_body(run_response).await;

    let claim_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/workers/worker-a/claim")
                .body(Body::empty())
                .expect("valid claim request"),
        )
        .await
        .expect("claim response");
    assert_eq!(claim_response.status(), StatusCode::OK);

    let fail_body = CompleteBuildRequest {
        status: WorkerBuildStatus::Failed,
        log_line: Some("step failed".to_string()),
    };

    let complete_response = app
        .clone()
        .oneshot(json_request(
            &format!("/workers/worker-a/builds/{}/complete", run_payload.build.id),
            &fail_body,
        ))
        .await
        .expect("complete response");
    assert_eq!(complete_response.status(), StatusCode::OK);

    let mut claimed_build_id = None;
    for _ in 0..20 {
        let claim_again = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/workers/worker-b/claim")
                    .body(Body::empty())
                    .expect("valid claim request"),
            )
            .await
            .expect("claim response");
        assert_eq!(claim_again.status(), StatusCode::OK);
        let claimed_again: ClaimBuildResponse = read_json_body(claim_again).await;

        if let Some(build) = claimed_again.build {
            claimed_build_id = Some(build.id);
            break;
        }

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    assert_eq!(
        claimed_build_id.expect("requeued build should exist"),
        run_payload.build.id
    );

    let metrics_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/metrics")
                .body(Body::empty())
                .expect("valid metrics request"),
        )
        .await
        .expect("metrics response");
    assert_eq!(metrics_response.status(), StatusCode::OK);
    let metrics: RuntimeMetricsResponse = read_json_body(metrics_response).await;
    assert_eq!(metrics.retry_requeued_total, 1);
}

#[tokio::test]
async fn exhausted_retries_moves_build_to_dead_letter_and_exposes_it() {
    let state = tardigrade_api::ApiState::with_components_and_mode_and_settings(
        "tardigrade-ci-test",
        Arc::new(InMemoryStorage::default()),
        Arc::new(InMemoryScheduler::default()),
        false,
        ServiceSettings {
            worker_lease_timeout_secs: 30,
            max_retries: 0,
            retry_backoff_ms: 0,
        },
    );
    let app = tardigrade_api::build_router(state);

    let create_body = CreateJobRequestBody {
        name: "build-dead-letter-check".to_string(),
        repository_url: "https://example.com/dlq.git".to_string(),
        pipeline_path: "pipelines/dlq.yml".to_string(),
        pipeline_yaml: None,
    };

    let create_response = app
        .clone()
        .oneshot(json_request("/jobs", &create_body))
        .await
        .expect("create response");
    let created: CreateJobResponse = read_json_body(create_response).await;

    let run_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/jobs/{}/run", created.job.id))
                .body(Body::empty())
                .expect("valid run request"),
        )
        .await
        .expect("run response");
    let run_payload: RunJobResponse = read_json_body(run_response).await;

    let claim_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/workers/worker-a/claim")
                .body(Body::empty())
                .expect("valid claim request"),
        )
        .await
        .expect("claim response");
    assert_eq!(claim_response.status(), StatusCode::OK);

    let fail_body = CompleteBuildRequest {
        status: WorkerBuildStatus::Failed,
        log_line: Some("fatal step failed".to_string()),
    };

    let complete_response = app
        .clone()
        .oneshot(json_request(
            &format!("/workers/worker-a/builds/{}/complete", run_payload.build.id),
            &fail_body,
        ))
        .await
        .expect("complete response");
    assert_eq!(complete_response.status(), StatusCode::OK);

    let dead_letter_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dead-letter-builds")
                .body(Body::empty())
                .expect("valid dead-letter request"),
        )
        .await
        .expect("dead-letter response");
    assert_eq!(dead_letter_response.status(), StatusCode::OK);
    let dead_letters: DeadLetterBuildsResponse = read_json_body(dead_letter_response).await;
    assert!(
        dead_letters
            .builds
            .iter()
            .any(|b| b.id == run_payload.build.id && b.status == JobStatus::Failed)
    );

    let metrics_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/metrics")
                .body(Body::empty())
                .expect("valid metrics request"),
        )
        .await
        .expect("metrics response");
    assert_eq!(metrics_response.status(), StatusCode::OK);
    let metrics: RuntimeMetricsResponse = read_json_body(metrics_response).await;
    assert_eq!(metrics.dead_letter_total, 1);
}

fn json_request(path: &str, payload: &impl Serialize) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(payload).expect("request body to json"),
        ))
        .expect("valid post request")
}

async fn read_json_body<T>(response: axum::response::Response) -> T
where
    T: DeserializeOwned,
{
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    serde_json::from_slice(&bytes).expect("json body")
}
