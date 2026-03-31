use async_graphql::SimpleObject;

use crate::WorkerInfo;

/// GraphQL projection for worker telemetry card.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlWorkerInfo {
    pub(crate) id: String,
    pub(crate) active_builds: usize,
    pub(crate) status: String,
    pub(crate) last_seen_at: String,
}

impl From<WorkerInfo> for GqlWorkerInfo {
    /// Converts one worker info payload into GraphQL projection.
    fn from(value: WorkerInfo) -> Self {
        Self {
            id: value.id,
            active_builds: value.active_builds,
            status: value.status,
            last_seen_at: value.last_seen_at.to_rfc3339(),
        }
    }
}
