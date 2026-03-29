use anyhow::Result;
use reqwest::Client;
use std::time::Duration;
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

/// Runs polling worker loop against API claim/complete endpoints.
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = load_worker_config();
    let server_url = config.server_url;
    let worker_id = config.worker_id;
    let poll_ms = config.poll_ms;

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

#[cfg(test)]
mod tests {
    use super::{parse_poll_ms, resolve_server_url, resolve_worker_id};

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
}
