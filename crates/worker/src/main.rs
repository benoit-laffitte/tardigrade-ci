use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;
use tardigrade_core::BuildRecord;
use tardigrade_api::{ClaimBuildResponse, CompleteBuildRequest, WorkerBuildStatus};
use tracing::{error, info};

#[derive(Debug, Clone)]
struct WorkerConfig {
    server_url: String,
    worker_id: String,
    poll_ms: u64,
}

fn resolve_server_url(raw: Option<&str>) -> String {
    raw.unwrap_or("http://127.0.0.1:8080").to_string()
}

fn resolve_worker_id(raw: Option<&str>) -> String {
    raw.unwrap_or("worker-local").to_string()
}

fn parse_poll_ms(raw: Option<&str>) -> u64 {
    raw.and_then(|v| v.parse::<u64>().ok()).unwrap_or(250)
}

fn load_worker_config() -> WorkerConfig {
    WorkerConfig {
        server_url: resolve_server_url(std::env::var("TARDIGRADE_SERVER_URL").ok().as_deref()),
        worker_id: resolve_worker_id(std::env::var("TARDIGRADE_WORKER_ID").ok().as_deref()),
        poll_ms: parse_poll_ms(std::env::var("TARDIGRADE_WORKER_POLL_MS").ok().as_deref()),
    }
}

fn claim_url(server_url: &str, worker_id: &str) -> String {
    format!("{server_url}/workers/{worker_id}/claim")
}

fn complete_url(server_url: &str, worker_id: &str, build_id: uuid::Uuid) -> String {
    format!("{server_url}/workers/{worker_id}/builds/{build_id}/complete")
}

fn completion_body() -> CompleteBuildRequest {
    CompleteBuildRequest {
        status: WorkerBuildStatus::Success,
        log_line: Some("Completed by tardigrade-worker".to_string()),
    }
}

enum ClaimStep {
    Retry,
    NoBuild,
    Build(BuildRecord),
}

#[async_trait]
trait WorkerApi {
    async fn claim(&self, claim_url: &str) -> Result<ClaimBuildResponse>;
    async fn complete(&self, complete_url: &str, body: &CompleteBuildRequest) -> Result<()>;
}

struct HttpWorkerApi {
    client: Client,
}

impl HttpWorkerApi {
    fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl WorkerApi for HttpWorkerApi {
    async fn claim(&self, claim_url: &str) -> Result<ClaimBuildResponse> {
        let payload = self
            .client
            .post(claim_url)
            .send()
            .await?
            .error_for_status()?
            .json::<ClaimBuildResponse>()
            .await?;
        Ok(payload)
    }

    async fn complete(&self, complete_url: &str, body: &CompleteBuildRequest) -> Result<()> {
        self.client
            .post(complete_url)
            .json(body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

async fn claim_step(api: &impl WorkerApi, claim_url: &str) -> ClaimStep {
    match api.claim(claim_url).await {
        Ok(payload) => match payload.build {
            Some(build) => ClaimStep::Build(build),
            None => ClaimStep::NoBuild,
        },
        Err(err) => {
            error!(error = %err, "claim request failed");
            ClaimStep::Retry
        }
    }
}

async fn complete_step(api: &impl WorkerApi, complete_url: &str, body: &CompleteBuildRequest) -> bool {
    match api.complete(complete_url, body).await {
        Ok(()) => true,
        Err(err) => {
            error!(error = %err, "complete request failed");
            false
        }
    }
}

/// Runs polling worker loop against API claim/complete endpoints.
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = load_worker_config();
    let server_url = config.server_url;
    let worker_id = config.worker_id;
    let poll_ms = config.poll_ms;

    let claim_url = claim_url(&server_url, &worker_id);
    let api = HttpWorkerApi::new(Client::new());

    info!(%server_url, %worker_id, poll_ms, "worker started");

    // Long-running control loop: claim -> execute -> complete.
    loop {
        let build = match claim_step(&api, &claim_url).await {
            ClaimStep::Build(build) => build,
            ClaimStep::NoBuild | ClaimStep::Retry => {
                // No work or claim failure: back off polling to reduce API pressure.
                tokio::time::sleep(Duration::from_millis(poll_ms)).await;
                continue;
            }
        };

        info!(build_id = %build.id, "claimed build");

        // Placeholder execution: this worker simulates a successful run.
        tokio::time::sleep(Duration::from_millis(75)).await;

        let complete_url = complete_url(&server_url, &worker_id, build.id);
        let body = completion_body();

        if complete_step(&api, &complete_url, &body).await {
            info!(build_id = %build.id, "build completed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ClaimStep, WorkerApi, claim_step, claim_url, complete_step, complete_url,
        completion_body, load_worker_config, parse_poll_ms, resolve_server_url,
        resolve_worker_id,
    };
    use anyhow::{Result, anyhow};
    use async_trait::async_trait;
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use tardigrade_api::ClaimBuildResponse;
    use tardigrade_core::BuildRecord;
    use tardigrade_api::WorkerBuildStatus;
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
        assert_eq!(resolve_server_url(Some("http://ci.internal:8080")), "http://ci.internal:8080");
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
        let build_id = Uuid::parse_str("00000000-0000-0000-0000-000000000123")
            .expect("valid uuid");

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
}
