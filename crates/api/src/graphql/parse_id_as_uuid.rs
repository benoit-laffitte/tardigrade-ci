use async_graphql::{Error as GraphQLError, ID};
use uuid::Uuid;

/// Parses one GraphQL ID as UUID for domain operations.
pub(crate) fn parse_id_as_uuid(id: &ID) -> Result<Uuid, GraphQLError> {
    Uuid::parse_str(id.as_str()).map_err(|_| GraphQLError::new("invalid UUID id"))
}
