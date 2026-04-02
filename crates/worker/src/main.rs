use anyhow::Result;
use reqwest::Client;
use std::time::Duration;
use tracing::info;

mod completion_payload;
mod endpoint_urls;
mod worker_api;
mod worker_config;
mod worker_steps;

pub(crate) use completion_payload::completion_body;
pub(crate) use endpoint_urls::{claim_url, complete_url};
pub(crate) use worker_api::{HttpWorkerApi, WorkerApi};
use worker_config::load_worker_config;
pub(crate) use worker_steps::{ClaimStep, claim_step, complete_step};

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

