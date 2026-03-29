use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde::{Serialize, de::DeserializeOwned};
use tardigrade_api::{CreateJobResponse, ListBuildsResponse, ListJobsResponse, RunJobResponse};
use tardigrade_scheduler::InMemoryScheduler;
use tardigrade_storage::PostgresStorage;
use tower::ServiceExt;
use std::sync::Arc;

#[derive(Serialize)]
struct CreateJobRequestBody {
    name: String,
    repository_url: String,
    pipeline_path: String,
}

#[tokio::test]
async fn postgres_storage_persists_jobs_and_builds_across_state_recreation() {
    let Some(database_url) = std::env::var("TARDIGRADE_TEST_DATABASE_URL").ok() else {
        eprintln!("skipping postgres persistence test: TARDIGRADE_TEST_DATABASE_URL not set");
        return;
    };

    let storage = Arc::new(
        PostgresStorage::connect(&database_url)
            .await
            .expect("connect postgres storage"),
    );
    let scheduler = Arc::new(InMemoryScheduler::default());

    let app1 = tardigrade_api::build_router(tardigrade_api::ApiState::with_components_and_mode(
        "tardigrade-ci-test",
        storage.clone(),
        scheduler,
        false,
    ));

    let create_body = CreateJobRequestBody {
        name: format!("persist-job-{}", uuid::Uuid::new_v4()),
        repository_url: "https://example.com/repo.git".to_string(),
        pipeline_path: "pipeline.yml".to_string(),
    };

    let create_response = app1
        .clone()
        .oneshot(json_request("/jobs", &create_body))
        .await
        .expect("create response");
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created: CreateJobResponse = read_json_body(create_response).await;

    let run_response = app1
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

    let storage_after_restart = Arc::new(
        PostgresStorage::connect(&database_url)
            .await
            .expect("reconnect postgres storage"),
    );

    let app2 = tardigrade_api::build_router(tardigrade_api::ApiState::with_components_and_mode(
        "tardigrade-ci-test",
        storage_after_restart,
        Arc::new(InMemoryScheduler::default()),
        false,
    ));

    let jobs_response = app2
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/jobs")
                .body(Body::empty())
                .expect("valid list jobs request"),
        )
        .await
        .expect("jobs response");
    assert_eq!(jobs_response.status(), StatusCode::OK);
    let jobs_payload: ListJobsResponse = read_json_body(jobs_response).await;
    assert!(jobs_payload.jobs.iter().any(|j| j.id == created.job.id));

    let builds_response = app2
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/builds")
                .body(Body::empty())
                .expect("valid list builds request"),
        )
        .await
        .expect("builds response");
    assert_eq!(builds_response.status(), StatusCode::OK);
    let builds_payload: ListBuildsResponse = read_json_body(builds_response).await;
    assert!(builds_payload.builds.iter().any(|b| b.id == run_payload.build.id));
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
