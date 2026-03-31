use anyhow::{Result, anyhow};
use tardigrade_core::JobStatus;

/// Maps domain status enum to compact persisted text representation.
pub(crate) fn status_to_str(status: &JobStatus) -> &'static str {
    // Storage uses normalized lowercase values to stay backend-agnostic.
    match status {
        JobStatus::Pending => "pending",
        JobStatus::Running => "running",
        JobStatus::Success => "success",
        JobStatus::Failed => "failed",
        JobStatus::Canceled => "canceled",
    }
}

/// Parses persisted text representation back into domain status enum.
pub(crate) fn parse_status(raw: &str) -> Result<JobStatus> {
    // Reject unknown states to avoid silently corrupting runtime behavior.
    match raw {
        "pending" => Ok(JobStatus::Pending),
        "running" => Ok(JobStatus::Running),
        "success" => Ok(JobStatus::Success),
        "failed" => Ok(JobStatus::Failed),
        "canceled" => Ok(JobStatus::Canceled),
        other => Err(anyhow!("unknown job status in storage: {other}")),
    }
}
