use anyhow::Result;
use reqwest::Client;
use std::time::Duration;
use tardigrade_api::{ClaimBuildResponse, CompleteBuildRequest, WorkerBuildStatus};
use tracing::{error, info};

/// Runs polling worker loop against API claim/complete endpoints.
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let server_url =
        std::env::var("TARDIGRADE_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let worker_id = std::env::var("TARDIGRADE_WORKER_ID").unwrap_or_else(|_| "worker-local".to_string());
    let poll_ms = std::env::var("TARDIGRADE_WORKER_POLL_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(250);

    let claim_url = format!("{server_url}/workers/{worker_id}/claim");
    let client = Client::new();

    info!(%server_url, %worker_id, poll_ms, "worker started");

    // Long-running control loop: claim -> execute -> complete.
    loop {
        let claimed = match client.post(&claim_url).send().await {
            Ok(resp) => match resp.error_for_status() {
                Ok(ok) => match ok.json::<ClaimBuildResponse>().await {
                    Ok(payload) => payload.build,
                    Err(err) => {
                        error!(error = %err, "failed to decode claim response");
                        tokio::time::sleep(Duration::from_millis(poll_ms)).await;
                        continue;
                    }
                },
                Err(err) => {
                    error!(error = %err, "claim request failed");
                    tokio::time::sleep(Duration::from_millis(poll_ms)).await;
                    continue;
                }
            },
            Err(err) => {
                error!(error = %err, "claim transport error");
                tokio::time::sleep(Duration::from_millis(poll_ms)).await;
                continue;
            }
        };

        let Some(build) = claimed else {
            // No work available: back off polling to reduce API pressure.
            tokio::time::sleep(Duration::from_millis(poll_ms)).await;
            continue;
        };

        info!(build_id = %build.id, "claimed build");

        // Placeholder execution: this worker simulates a successful run.
        tokio::time::sleep(Duration::from_millis(75)).await;

        let complete_url = format!("{server_url}/workers/{worker_id}/builds/{}/complete", build.id);
        let body = CompleteBuildRequest {
            status: WorkerBuildStatus::Success,
            log_line: Some("Completed by tardigrade-worker".to_string()),
        };

        match client.post(&complete_url).json(&body).send().await {
            Ok(resp) => {
                if let Err(err) = resp.error_for_status() {
                    error!(build_id = %build.id, error = %err, "complete request failed");
                } else {
                    info!(build_id = %build.id, "build completed");
                }
            }
            Err(err) => {
                error!(build_id = %build.id, error = %err, "complete transport error");
            }
        }
    }
}
