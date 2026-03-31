use async_graphql::Enum;
use tardigrade_core::JobStatus;

/// GraphQL enum mirroring runtime build lifecycle statuses.
#[derive(Clone, Copy, Eq, PartialEq, Enum)]
pub(crate) enum GqlJobStatus {
    Pending,
    Running,
    Success,
    Failed,
    Canceled,
}

impl From<JobStatus> for GqlJobStatus {
    /// Converts core build status into GraphQL enum value.
    fn from(value: JobStatus) -> Self {
        match value {
            JobStatus::Pending => Self::Pending,
            JobStatus::Running => Self::Running,
            JobStatus::Success => Self::Success,
            JobStatus::Failed => Self::Failed,
            JobStatus::Canceled => Self::Canceled,
        }
    }
}
