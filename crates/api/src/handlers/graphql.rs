use async_graphql::http::{GraphQLPlaygroundConfig, playground_source};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{Extension, response::Html};

use crate::{ApiAuthContext, CiGraphQLSchema};

/// Serves GraphQL playground for interactive schema exploration.
pub(crate) async fn graphql_playground() -> Html<String> {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

/// Executes one GraphQL request against CI schema.
pub(crate) async fn graphql_handler(
    Extension(schema): Extension<CiGraphQLSchema>,
    auth_context: Option<Extension<ApiAuthContext>>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut request = req.into_inner();
    if let Some(Extension(context)) = auth_context {
        request = request.data(context);
    } else {
        request = request.data(ApiAuthContext::default());
    }

    schema.execute(request).await.into()
}
