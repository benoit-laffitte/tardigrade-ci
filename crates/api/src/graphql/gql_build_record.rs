use async_graphql::{ID, SimpleObject};
use tardigrade_core::BuildRecord;

use super::GqlJobStatus;

/// GraphQL projection for persisted build records.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlBuildRecord {
    pub(crate) id: ID,
    pub(crate) job_id: ID,
    pub(crate) status: GqlJobStatus,
    pub(crate) queued_at: String,
    pub(crate) started_at: Option<String>,
    pub(crate) finished_at: Option<String>,
    pub(crate) logs: Vec<String>,
}

impl From<BuildRecord> for GqlBuildRecord {
    /// Converts one domain build record into GraphQL projection.
    fn from(value: BuildRecord) -> Self {
        Self {
            id: ID(value.id.to_string()),
            job_id: ID(value.job_id.to_string()),
            status: value.status.into(),
            queued_at: value.queued_at.to_rfc3339(),
            started_at: value.started_at.map(|dt| dt.to_rfc3339()),
            finished_at: value.finished_at.map(|dt| dt.to_rfc3339()),
            logs: value.logs,
        }
    }
}
