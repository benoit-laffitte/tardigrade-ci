use async_graphql::Enum;
use tardigrade_core::ScmProvider;

/// GraphQL enum mirroring supported SCM providers.
#[derive(Clone, Copy, Eq, PartialEq, Enum)]
pub(crate) enum GqlScmProvider {
    Github,
    Gitlab,
}

impl From<GqlScmProvider> for ScmProvider {
    /// Converts GraphQL SCM provider enum into the core domain enum.
    fn from(value: GqlScmProvider) -> Self {
        match value {
            GqlScmProvider::Github => ScmProvider::Github,
            GqlScmProvider::Gitlab => ScmProvider::Gitlab,
        }
    }
}
