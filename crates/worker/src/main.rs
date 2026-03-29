use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;
use tardigrade_api::{ClaimBuildResponse, CompleteBuildRequest, WorkerBuildStatus};
use tardigrade_core::BuildRecord;
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

async fn complete_step(
    api: &impl WorkerApi,
    complete_url: &str,
    body: &CompleteBuildRequest,
) -> bool {
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
mod tests;
