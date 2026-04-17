use serde::Deserialize;

/// Runtime tuning knobs for reliability behavior (leases/retries/backoff).
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default)]
pub struct ServiceSettings {
    pub worker_lease_timeout_secs: u64,
    pub max_retries: u32,
    pub retry_backoff_ms: u64,
    pub webhook_dedup_ttl_secs: u64,
}

impl Default for ServiceSettings {
    /// Provides safe baseline reliability settings when TOML omits explicit values.
    fn default() -> Self {
        Self {
            worker_lease_timeout_secs: 30,
            max_retries: 2,
            retry_backoff_ms: 1000,
            webhook_dedup_ttl_secs: 3600,
        }
    }
}
