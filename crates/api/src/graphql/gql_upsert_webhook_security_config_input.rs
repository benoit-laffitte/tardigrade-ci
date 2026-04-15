use async_graphql::InputObject;

use super::GqlScmProvider;

/// GraphQL input used to upsert webhook verification settings for one repository.
#[derive(InputObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlUpsertWebhookSecurityConfigInput {
    pub(crate) repository_url: String,
    pub(crate) provider: GqlScmProvider,
    pub(crate) secret: String,
    pub(crate) allowed_ips: Vec<String>,
}
