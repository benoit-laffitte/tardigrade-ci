use tardigrade_api::CompleteBuildRequest;
use tardigrade_core::BuildRecord;
use tracing::error;

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

/// Executes one claim attempt and maps transport/result state to loop step.
pub(crate) async fn claim_step(api: &impl WorkerApi, claim_url: &str) -> ClaimStep {
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

/// Executes one completion call and reports success/failure to loop controller.
pub(crate) async fn complete_step(
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
