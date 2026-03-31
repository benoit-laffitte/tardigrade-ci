use async_graphql::SimpleObject;

/// GraphQL projection for health endpoint response.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlHealthResponse {
    pub(crate) status: String,
    pub(crate) service: String,
}
