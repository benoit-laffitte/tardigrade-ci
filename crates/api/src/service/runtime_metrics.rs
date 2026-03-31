/// Mutable runtime counters for reliability and SCM trigger observability.
#[derive(Default)]
pub(crate) struct RuntimeMetrics {
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
