use async_graphql::SimpleObject;

/// GraphQL projection for readiness endpoint response.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlReadyResponse {
    pub(crate) status: String,
}
