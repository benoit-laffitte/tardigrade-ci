use tardigrade_api::CompleteBuildRequest;
use tardigrade_core::BuildRecord;
use tracing::error;
use uuid::Uuid;

use crate::WorkerApi;

/// Result of one claim attempt in worker control loop.
pub(crate) enum ClaimStep {
    /// Retry claim after backoff because request failed.
    Retry,
    /// No build available in queue.
    NoBuild,
    /// One build was successfully claimed.
    Build(BuildRecord),
}

/// Executes one GraphQL claim attempt and maps transport or result state to loop step.
pub(crate) async fn claim_step(
    api: &impl WorkerApi,
    graphql_url: &str,
    worker_id: &str,
) -> ClaimStep {
    match api.claim(graphql_url, worker_id).await {
        Ok(build) => match build {
            Some(build) => ClaimStep::Build(build),
            None => ClaimStep::NoBuild,
        },
        Err(err) => {
            error!(error = %err, "claim request failed");
            ClaimStep::Retry
        }
    }
}

/// Executes one GraphQL completion call and reports success or failure to loop controller.
pub(crate) async fn complete_step(
    api: &impl WorkerApi,
    graphql_url: &str,
    worker_id: &str,
    build_id: Uuid,
    body: &CompleteBuildRequest,
) -> bool {
    match api.complete(graphql_url, worker_id, build_id, body).await {
        Ok(()) => true,
        Err(err) => {
            error!(error = %err, "complete request failed");
            false
        }
    }
}

#[cfg(test)]
mod tests;
