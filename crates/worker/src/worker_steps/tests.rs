use super::{ClaimStep, claim_step, complete_step};
use crate::{WorkerApi, complete_url, completion_body};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Mutex;
use tardigrade_api::{ClaimBuildResponse, WorkerBuildStatus};
use tardigrade_core::BuildRecord;
use uuid::Uuid;

/// Captures one completion API call for assertion in tests.
#[derive(Debug, Clone)]
struct CompleteCall {
    /// Completion URL requested by worker step.
    url: String,
    /// Indicates success status was requested.
    status_is_success: bool,
    /// Optional completion log line sent by worker.
    log_line: Option<String>,
}

/// Mock API transport that replays claim/complete outcomes deterministically.
struct MockWorkerApi {
    /// Queued claim outcomes consumed in FIFO order.
    claim_results: Mutex<VecDeque<Result<ClaimBuildResponse>>>,
    /// Queued completion outcomes consumed in FIFO order.
    complete_results: Mutex<VecDeque<Result<()>>>,
    /// Captured completion calls for assertion.
    complete_calls: Mutex<Vec<CompleteCall>>,
}

impl MockWorkerApi {
    /// Builds mock transport preloaded with claim outcomes.
    fn with_claim_results(claim_results: Vec<Result<ClaimBuildResponse>>) -> Self {
        Self {
            claim_results: Mutex::new(claim_results.into()),
            complete_results: Mutex::new(VecDeque::new()),
            complete_calls: Mutex::new(Vec::new()),
        }
    }

    /// Builds mock transport preloaded with completion outcomes.
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
    /// Returns next mocked claim payload or error.
    async fn claim(&self, _claim_url: &str) -> Result<ClaimBuildResponse> {
        self.claim_results
            .lock()
            .expect("claim results poisoned")
            .pop_front()
            .unwrap_or_else(|| Err(anyhow!("no mocked claim result")))
    }

    /// Records completion request and returns next mocked completion result.
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

/// Confirms claim step retries on claim transport failure.
#[tokio::test]
async fn claim_step_returns_retry_on_claim_error() {
    let api = MockWorkerApi::with_claim_results(vec![Err(anyhow!("boom"))]);
    let step = claim_step(&api, "http://127.0.0.1:8080/workers/worker-a/claim").await;
    assert!(matches!(step, ClaimStep::Retry));
}

/// Confirms claim step returns NoBuild when queue is empty.
#[tokio::test]
async fn claim_step_returns_no_build_when_queue_is_empty() {
    let api = MockWorkerApi::with_claim_results(vec![Ok(ClaimBuildResponse { build: None })]);
    let step = claim_step(&api, "http://127.0.0.1:8080/workers/worker-a/claim").await;
    assert!(matches!(step, ClaimStep::NoBuild));
}

/// Confirms claim step returns claimed build when payload carries one.
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

/// Confirms complete step reports success and records request payload.
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

/// Confirms complete step reports failure when API completion fails.
#[tokio::test]
async fn complete_step_reports_failure_on_error() {
    let api = MockWorkerApi::with_complete_results(vec![Err(anyhow!("network"))]);
    let build_id = Uuid::new_v4();
    let url = complete_url("http://127.0.0.1:8080", "worker-a", build_id);
    let body = completion_body();

    let success = complete_step(&api, &url, &body).await;
    assert!(!success);
}
