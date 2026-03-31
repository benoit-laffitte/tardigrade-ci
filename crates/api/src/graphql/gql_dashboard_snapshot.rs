use async_graphql::SimpleObject;

use super::{GqlBuildRecord, GqlJobDefinition, GqlRuntimeMetrics, GqlWorkerInfo};

/// GraphQL projection grouping dashboard panels into a single payload.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlDashboardSnapshot {
    pub(crate) jobs: Vec<GqlJobDefinition>,
    pub(crate) builds: Vec<GqlBuildRecord>,
    pub(crate) workers: Vec<GqlWorkerInfo>,
    pub(crate) metrics: GqlRuntimeMetrics,
    pub(crate) dead_letter_builds: Vec<GqlBuildRecord>,
}
