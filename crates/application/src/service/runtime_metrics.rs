/// Mutable runtime counters for reliability and SCM trigger observability.
#[derive(Default)]
pub struct RuntimeMetrics {
    pub reclaimed_total: u64,
    pub retry_requeued_total: u64,
    pub ownership_conflicts_total: u64,
    pub dead_letter_total: u64,
    pub scm_webhook_received_total: u64,
    pub scm_webhook_accepted_total: u64,
    pub scm_webhook_rejected_total: u64,
    pub scm_webhook_duplicate_total: u64,
    pub scm_trigger_enqueued_builds_total: u64,
    pub scm_polling_ticks_total: u64,
    pub scm_polling_repositories_total: u64,
    pub scm_polling_enqueued_builds_total: u64,
}
