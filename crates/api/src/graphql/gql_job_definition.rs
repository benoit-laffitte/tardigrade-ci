use async_graphql::{ID, SimpleObject};
use tardigrade_core::JobDefinition;

/// GraphQL projection for persisted job definitions.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlJobDefinition {
    pub(crate) id: ID,
    pub(crate) name: String,
    pub(crate) repository_url: String,
    pub(crate) pipeline_path: String,
    pub(crate) created_at: String,
    /// Pipeline YAML inline ou None (si pipeline_path utilisé)
    pub(crate) pipeline_content: Option<String>,
}

impl From<JobDefinition> for GqlJobDefinition {
    /// Converts one domain job definition into GraphQL projection.
    fn from(value: JobDefinition) -> Self {
        Self {
            id: ID(value.id.to_string()),
            name: value.name,
            repository_url: value.repository_url,
            pipeline_path: value.pipeline_path,
            created_at: value.created_at.to_rfc3339(),
            pipeline_content: value.pipeline_content,
        }
    }
}
