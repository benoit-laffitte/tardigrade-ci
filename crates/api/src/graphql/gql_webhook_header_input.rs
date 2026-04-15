use async_graphql::InputObject;

/// GraphQL input used to reconstruct one inbound SCM webhook header.
#[derive(InputObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlWebhookHeaderInput {
    pub(crate) name: String,
    pub(crate) value: String,
}
