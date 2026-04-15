use async_graphql::{EmptySubscription, Schema};
use axum::{Extension, Router, routing::get};

use crate::ApiState;
use crate::graphql::{MutationRoot, QueryRoot};
use crate::handlers::{graphql_handler, graphql_playground};

/// Builds the full HTTP router for CI control-plane API.
pub fn build_router(state: ApiState) -> Router {
    let graphql_schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(state.clone())
        .finish();

    // Router exposes only the GraphQL control-plane surface.
    Router::new()
        .route("/graphql", get(graphql_playground).post(graphql_handler))
        .layer(Extension(graphql_schema))
        .with_state(state)
}
