use async_graphql::InputObject;

/// GraphQL input used by create_job mutation.
#[derive(InputObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlCreateJobInput {
    pub(crate) name: String,
    pub(crate) repository_url: String,
    pub(crate) pipeline_path: String,
    pub(crate) pipeline_yaml: Option<String>,
}
