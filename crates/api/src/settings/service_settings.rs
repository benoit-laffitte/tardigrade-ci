/// Runtime tuning knobs for reliability behavior (leases/retries/backoff).
#[derive(Clone, Copy)]
pub struct ServiceSettings {
    pub worker_lease_timeout_secs: u64,
    pub max_retries: u32,
    pub retry_backoff_ms: u64,
    pub webhook_dedup_ttl_secs: u64,
}

impl ServiceSettings {
    /// Loads reliability settings from environment variables with safe defaults.
    pub fn from_env() -> Self {
        // Env-based defaults keep local dev easy while allowing production tuning.
        let worker_lease_timeout_secs = std::env::var("TARDIGRADE_WORKER_LEASE_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);
        let max_retries = std::env::var("TARDIGRADE_BUILD_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(2);
        let retry_backoff_ms = std::env::var("TARDIGRADE_BUILD_RETRY_BACKOFF_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1000);
        let webhook_dedup_ttl_secs = std::env::var("TARDIGRADE_SCM_WEBHOOK_DEDUP_TTL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(3600);

        Self {
            worker_lease_timeout_secs,
            max_retries,
            retry_backoff_ms,
            webhook_dedup_ttl_secs,
        }
    }
}
