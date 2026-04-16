use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

/// Worker runtime configuration resolved from TOML file.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub(crate) struct WorkerConfig {
    /// Base URL of the API server to poll.
    pub(crate) server_url: String,
    /// Stable worker identifier sent to claim/complete endpoints.
    pub(crate) worker_id: String,
    /// Poll interval in milliseconds for claim loop backoff.
    pub(crate) poll_ms: u64,
    /// Enables HTTP/2-specific transport tuning for worker requests.
    pub(crate) http2_enabled: bool,
    /// Enables h2c prior knowledge mode for cleartext HTTP/2 deployments.
    pub(crate) http2_prior_knowledge: bool,
    /// Per-request timeout in seconds for GraphQL claim/complete mutations.
    pub(crate) request_timeout_secs: u64,
    /// Idle pooled connection timeout in seconds.
    pub(crate) pool_idle_timeout_secs: u64,
    /// Maximum idle pooled connections kept per host.
    pub(crate) pool_max_idle_per_host: usize,
    /// HTTP/2 keep-alive ping interval in seconds.
    pub(crate) http2_keep_alive_secs: u64,
}

impl Default for WorkerConfig {
    /// Provides stable defaults for local worker runs when TOML omits explicit values.
    fn default() -> Self {
        Self {
            server_url: "http://127.0.0.1:8080".to_string(),
            worker_id: "worker-local".to_string(),
            poll_ms: 250,
            http2_enabled: true,
            http2_prior_knowledge: false,
            request_timeout_secs: 30,
            pool_idle_timeout_secs: 90,
            pool_max_idle_per_host: 32,
            http2_keep_alive_secs: 30,
        }
    }
}

/// Top-level worker config file shape.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct WorkerConfigFile {
    worker: WorkerConfig,
}

/// Loads worker configuration from one TOML file path.
pub(crate) fn load_worker_config(path: &str) -> Result<WorkerConfig> {
    let raw = fs::read_to_string(path).with_context(|| format!("read config file at {path}"))?;
    let parsed: WorkerConfigFile =
        toml::from_str(&raw).with_context(|| format!("parse TOML from {path}"))?;

    let mut worker = parsed.worker;
    if worker.server_url.trim().is_empty() {
        worker.server_url = WorkerConfig::default().server_url;
    }
    if worker.worker_id.trim().is_empty() {
        worker.worker_id = WorkerConfig::default().worker_id;
    }
    if worker.poll_ms == 0 {
        worker.poll_ms = WorkerConfig::default().poll_ms;
    }
    if worker.request_timeout_secs == 0 {
        worker.request_timeout_secs = WorkerConfig::default().request_timeout_secs;
    }
    if worker.pool_idle_timeout_secs == 0 {
        worker.pool_idle_timeout_secs = WorkerConfig::default().pool_idle_timeout_secs;
    }
    if worker.pool_max_idle_per_host == 0 {
        worker.pool_max_idle_per_host = WorkerConfig::default().pool_max_idle_per_host;
    }
    if worker.http2_keep_alive_secs == 0 {
        worker.http2_keep_alive_secs = WorkerConfig::default().http2_keep_alive_secs;
    }

    Ok(worker)
}

#[cfg(test)]
mod tests;
