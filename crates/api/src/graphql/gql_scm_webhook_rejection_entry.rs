use async_graphql::SimpleObject;

use crate::ScmWebhookRejectionEntry;

/// GraphQL projection for recent SCM webhook rejection diagnostics.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlScmWebhookRejectionEntry {
    pub(crate) reason_code: String,
    pub(crate) provider: Option<String>,
    pub(crate) repository_url: Option<String>,
    pub(crate) at: String,
}

impl From<ScmWebhookRejectionEntry> for GqlScmWebhookRejectionEntry {
    /// Converts one rejection diagnostic entry into GraphQL shape.
    fn from(value: ScmWebhookRejectionEntry) -> Self {
        Self {
            reason_code: value.reason_code,
            provider: value.provider,
            repository_url: value.repository_url,
            at: value.at,
        }
    }
}
