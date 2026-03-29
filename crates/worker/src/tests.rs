use super::{
    ClaimStep, WorkerApi, claim_step, claim_url, complete_step, complete_url, completion_body,
    load_worker_config, parse_poll_ms, resolve_server_url, resolve_worker_id,
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Mutex;
use tardigrade_api::{ClaimBuildResponse, WorkerBuildStatus};
use tardigrade_core::BuildRecord;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct CompleteCall {
    url: String,
    status_is_success: bool,
    log_line: Option<String>,
}

struct MockWorkerApi {
    claim_results: Mutex<VecDeque<Result<ClaimBuildResponse>>>,
    complete_results: Mutex<VecDeque<Result<()>>>,
    complete_calls: Mutex<Vec<CompleteCall>>,
}

impl MockWorkerApi {
    fn with_claim_results(claim_results: Vec<Result<ClaimBuildResponse>>) -> Self {
        Self {
            claim_results: Mutex::new(claim_results.into()),
            complete_results: Mutex::new(VecDeque::new()),
            complete_calls: Mutex::new(Vec::new()),
        }
    }

    fn with_complete_results(complete_results: Vec<Result<()>>) -> Self {
        Self {
            claim_results: Mutex::new(VecDeque::new()),
            complete_results: Mutex::new(complete_results.into()),
            complete_calls: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl WorkerApi for MockWorkerApi {
    async fn claim(&self, _claim_url: &str) -> Result<ClaimBuildResponse> {
        self.claim_results
            .lock()
            .expect("claim results poisoned")
            .pop_front()
            .unwrap_or_else(|| Err(anyhow!("no mocked claim result")))
    }

    async fn complete(
        &self,
        complete_url: &str,
        body: &tardigrade_api::CompleteBuildRequest,
    ) -> Result<()> {
        self.complete_calls
            .lock()
            .expect("complete calls poisoned")
            .push(CompleteCall {
                url: complete_url.to_string(),
                status_is_success: matches!(body.status, WorkerBuildStatus::Success),
                log_line: body.log_line.clone(),
            });

        self.complete_results
            .lock()
            .expect("complete results poisoned")
            .pop_front()
            .unwrap_or_else(|| Err(anyhow!("no mocked complete result")))
    }
}

#[test]
fn worker_config_defaults_are_stable() {
    assert_eq!(resolve_server_url(None), "http://127.0.0.1:8080");
    assert_eq!(resolve_worker_id(None), "worker-local");
    assert_eq!(parse_poll_ms(None), 250);
}

#[test]
fn worker_config_uses_provided_values() {
    assert_eq!(
        resolve_server_url(Some("http://ci.internal:8080")),
        "http://ci.internal:8080"
    );
    assert_eq!(resolve_worker_id(Some("worker-a")), "worker-a");
    assert_eq!(parse_poll_ms(Some("500")), 500);
}

#[test]
fn worker_config_rejects_invalid_poll_value() {
    assert_eq!(parse_poll_ms(Some("not-a-number")), 250);
}

#[test]
fn load_worker_config_produces_valid_values() {
    let cfg = load_worker_config();
    assert!(!cfg.server_url.trim().is_empty());
    assert!(!cfg.worker_id.trim().is_empty());
    assert!(cfg.poll_ms > 0);
}

#[test]
fn worker_urls_are_built_consistently() {
    let server_url = "http://127.0.0.1:8080";
    let worker_id = "worker-a";
    let build_id = Uuid::parse_str("00000000-0000-0000-0000-000000000123").expect("valid uuid");

    assert_eq!(
        claim_url(server_url, worker_id),
        "http://127.0.0.1:8080/workers/worker-a/claim"
    );
    assert_eq!(
        complete_url(server_url, worker_id, build_id),
        "http://127.0.0.1:8080/workers/worker-a/builds/00000000-0000-0000-0000-000000000123/complete"
    );
}

#[test]
fn completion_payload_defaults_to_success_with_log_line() {
    let payload = completion_body();
    assert!(matches!(payload.status, WorkerBuildStatus::Success));
    assert_eq!(
        payload.log_line.as_deref(),
        Some("Completed by tardigrade-worker")
    );
}

#[tokio::test]
async fn claim_step_returns_retry_on_claim_error() {
    let api = MockWorkerApi::with_claim_results(vec![Err(anyhow!("boom"))]);
    let step = claim_step(&api, "http://127.0.0.1:8080/workers/worker-a/claim").await;
    assert!(matches!(step, ClaimStep::Retry));
}

#[tokio::test]
async fn claim_step_returns_no_build_when_queue_is_empty() {
    let api = MockWorkerApi::with_claim_results(vec![Ok(ClaimBuildResponse { build: None })]);
    let step = claim_step(&api, "http://127.0.0.1:8080/workers/worker-a/claim").await;
    assert!(matches!(step, ClaimStep::NoBuild));
}

#[tokio::test]
async fn claim_step_returns_build_when_payload_contains_one() {
    let build = BuildRecord::queued(Uuid::new_v4());
    let api = MockWorkerApi::with_claim_results(vec![Ok(ClaimBuildResponse {
        build: Some(build.clone()),
    })]);
    let step = claim_step(&api, "http://127.0.0.1:8080/workers/worker-a/claim").await;

    let ClaimStep::Build(claimed) = step else {
        panic!("expected build claim result");
    };
    assert_eq!(claimed.id, build.id);
}

#[tokio::test]
async fn complete_step_reports_success_and_captures_call() {
    let api = MockWorkerApi::with_complete_results(vec![Ok(())]);
    let build_id = Uuid::new_v4();
    let url = complete_url("http://127.0.0.1:8080", "worker-a", build_id);
    let body = completion_body();

    let success = complete_step(&api, &url, &body).await;
    assert!(success);

    let calls = api.complete_calls.lock().expect("complete calls poisoned");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].url, url);
    assert!(calls[0].status_is_success);
    assert_eq!(
        calls[0].log_line.as_deref(),
        Some("Completed by tardigrade-worker")
    );
}

#[tokio::test]
async fn complete_step_reports_failure_on_error() {
    let api = MockWorkerApi::with_complete_results(vec![Err(anyhow!("network"))]);
    let build_id = Uuid::new_v4();
    let url = complete_url("http://127.0.0.1:8080", "worker-a", build_id);
    let body = completion_body();

    let success = complete_step(&api, &url, &body).await;
    assert!(!success);
}
