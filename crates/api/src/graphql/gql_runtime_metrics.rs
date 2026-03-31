use async_graphql::SimpleObject;

use crate::RuntimeMetricsResponse;

/// GraphQL projection for runtime reliability counters.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlRuntimeMetrics {
    pub(crate) reclaimed_total: u64,
    pub(crate) retry_requeued_total: u64,
    pub(crate) ownership_conflicts_total: u64,
    pub(crate) dead_letter_total: u64,
    pub(crate) scm_webhook_received_total: u64,
    pub(crate) scm_webhook_accepted_total: u64,
    pub(crate) scm_webhook_rejected_total: u64,
    pub(crate) scm_webhook_duplicate_total: u64,
    pub(crate) scm_trigger_enqueued_builds_total: u64,
    pub(crate) scm_polling_ticks_total: u64,
    pub(crate) scm_polling_repositories_total: u64,
    pub(crate) scm_polling_enqueued_builds_total: u64,
}

impl From<RuntimeMetricsResponse> for GqlRuntimeMetrics {
    /// Converts runtime metrics payload into GraphQL projection.
    fn from(value: RuntimeMetricsResponse) -> Self {
        Self {
            reclaimed_total: value.reclaimed_total,
            retry_requeued_total: value.retry_requeued_total,
            ownership_conflicts_total: value.ownership_conflicts_total,
            dead_letter_total: value.dead_letter_total,
            scm_webhook_received_total: value.scm_webhook_received_total,
            scm_webhook_accepted_total: value.scm_webhook_accepted_total,
            scm_webhook_rejected_total: value.scm_webhook_rejected_total,
            scm_webhook_duplicate_total: value.scm_webhook_duplicate_total,
            scm_trigger_enqueued_builds_total: value.scm_trigger_enqueued_builds_total,
            scm_polling_ticks_total: value.scm_polling_ticks_total,
            scm_polling_repositories_total: value.scm_polling_repositories_total,
            scm_polling_enqueued_builds_total: value.scm_polling_enqueued_builds_total,
        }
    }
}
