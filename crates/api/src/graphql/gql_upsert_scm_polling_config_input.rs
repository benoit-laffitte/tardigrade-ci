use async_graphql::InputObject;

use super::GqlScmProvider;

/// GraphQL input used to upsert SCM polling settings for one repository.
#[derive(InputObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlUpsertScmPollingConfigInput {
    pub(crate) repository_url: String,
    pub(crate) provider: GqlScmProvider,
    pub(crate) enabled: bool,
    pub(crate) interval_secs: i32,
    pub(crate) branches: Vec<String>,
}
