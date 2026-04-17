/// SCM webhook event families that can trigger build enqueue logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScmTriggerEvent {
    Push,
    PullRequest,
    MergeRequest,
    Tag,
    ManualDispatch,
}
