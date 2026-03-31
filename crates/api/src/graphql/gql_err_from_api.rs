use async_graphql::{Error as GraphQLError, ErrorExtensions, Value as GraphQLValue};

use crate::ApiError;

/// Converts API errors into GraphQL transport errors with structured extensions.
pub(crate) fn gql_err_from_api(err: ApiError) -> GraphQLError {
    match err {
        ApiError::InvalidPipeline { message, details } => {
            GraphQLError::new(message).extend_with(|_, extensions| {
                extensions.set("code", "invalid_pipeline");
                if let Some(issues) = details {
                    let details_json = match serde_json::to_value(issues) {
                        Ok(value) => value,
                        Err(_) => serde_json::Value::Null,
                    };
                    if let Ok(details_value) = GraphQLValue::from_json(details_json) {
                        extensions.set("details", details_value);
                    }
                }
            })
        }
        _ => GraphQLError::new(format!(
            "request failed with status {}",
            err.status_code().as_u16()
        )),
    }
}
