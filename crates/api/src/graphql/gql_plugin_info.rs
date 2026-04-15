use async_graphql::SimpleObject;

use crate::PluginInfo;

/// GraphQL projection for plugin registry entries.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlPluginInfo {
    pub(crate) name: String,
    pub(crate) state: String,
    pub(crate) capabilities: Vec<String>,
    pub(crate) source_manifest_entry: String,
}

impl From<PluginInfo> for GqlPluginInfo {
    /// Converts one plugin inventory entry into GraphQL shape.
    fn from(value: PluginInfo) -> Self {
        Self {
            name: value.name,
            state: value.state,
            capabilities: value.capabilities,
            source_manifest_entry: value.source_manifest_entry,
        }
    }
}
