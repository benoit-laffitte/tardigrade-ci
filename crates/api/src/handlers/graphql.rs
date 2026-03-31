use async_graphql::http::{GraphQLPlaygroundConfig, playground_source};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{Extension, response::Html};

use crate::CiGraphQLSchema;

/// Serves GraphQL playground for interactive schema exploration.
pub(crate) async fn graphql_playground() -> Html<String> {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

/// Executes one GraphQL request against CI schema.
pub(crate) async fn graphql_handler(
    Extension(schema): Extension<CiGraphQLSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}
