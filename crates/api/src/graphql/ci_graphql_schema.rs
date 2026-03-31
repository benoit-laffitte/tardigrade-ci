use async_graphql::{EmptySubscription, Schema};

use super::{MutationRoot, QueryRoot};

/// GraphQL schema serving CI query and mutation operations.
pub(crate) type CiGraphQLSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;
