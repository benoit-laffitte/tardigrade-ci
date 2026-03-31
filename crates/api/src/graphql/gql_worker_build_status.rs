use async_graphql::Enum;

/// Worker-reported terminal status accepted by GraphQL completion mutation.
#[derive(Clone, Copy, Eq, PartialEq, Enum)]
pub(crate) enum GqlWorkerBuildStatus {
    Success,
    Failed,
}
