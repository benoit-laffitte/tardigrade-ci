use super::GqlScmProvider;
use async_graphql::SimpleObject;

/// GraphQL object for reading webhook security config for a repository/provider.
#[derive(SimpleObject, Clone)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlWebhookSecurityConfig {
    pub(crate) repository_url: String,
    pub(crate) provider: GqlScmProvider,
    pub(crate) secret_masked: String, // Never expose raw secret!
    pub(crate) allowed_ips: Vec<String>,
}
