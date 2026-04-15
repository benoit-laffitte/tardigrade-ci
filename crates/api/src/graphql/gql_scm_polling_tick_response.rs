use async_graphql::SimpleObject;

use crate::ScmPollingTickResponse;

/// GraphQL projection for one SCM polling tick execution result.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlScmPollingTickResponse {
    pub(crate) polled_repositories: usize,
    pub(crate) enqueued_builds: usize,
}

impl From<ScmPollingTickResponse> for GqlScmPollingTickResponse {
    /// Converts one polling tick response into GraphQL shape.
    fn from(value: ScmPollingTickResponse) -> Self {
        Self {
            polled_repositories: value.polled_repositories,
            enqueued_builds: value.enqueued_builds,
        }
    }
}
