use async_graphql::SimpleObject;

/// GraphQL projection for liveness endpoint response.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlLiveResponse {
    pub(crate) status: String,
}
