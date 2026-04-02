use tardigrade_api::{CompleteBuildRequest, WorkerBuildStatus};

/// Builds default completion payload used by simulated worker execution.
pub(crate) fn completion_body() -> CompleteBuildRequest {
    CompleteBuildRequest {
        status: WorkerBuildStatus::Success,
        log_line: Some("Completed by tardigrade-worker".to_string()),
    }
}

#[cfg(test)]
mod tests;
