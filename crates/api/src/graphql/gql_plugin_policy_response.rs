use async_graphql::SimpleObject;

use crate::PluginPolicyResponse;

/// GraphQL projection for granted plugin capabilities in one context.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlPluginPolicyResponse {
    pub(crate) context: String,
    pub(crate) granted_capabilities: Vec<String>,
}

impl From<PluginPolicyResponse> for GqlPluginPolicyResponse {
    /// Converts one plugin policy response into GraphQL shape.
    fn from(value: PluginPolicyResponse) -> Self {
        Self {
            context: value.context,
            granted_capabilities: value.granted_capabilities,
        }
    }
}
