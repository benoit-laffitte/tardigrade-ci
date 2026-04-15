use async_graphql::SimpleObject;

use crate::PluginAuthorizationCheckResponse;

/// GraphQL projection for plugin authorization dry-run results.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlPluginAuthorizationCheckResponse {
    pub(crate) plugin_name: String,
    pub(crate) context: String,
    pub(crate) required_capabilities: Vec<String>,
    pub(crate) granted_capabilities: Vec<String>,
    pub(crate) missing_capabilities: Vec<String>,
    pub(crate) allowed: bool,
}

impl From<PluginAuthorizationCheckResponse> for GqlPluginAuthorizationCheckResponse {
    /// Converts one plugin authorization decision into GraphQL shape.
    fn from(value: PluginAuthorizationCheckResponse) -> Self {
        Self {
            plugin_name: value.plugin_name,
            context: value.context,
            required_capabilities: value.required_capabilities,
            granted_capabilities: value.granted_capabilities,
            missing_capabilities: value.missing_capabilities,
            allowed: value.allowed,
        }
    }
}
