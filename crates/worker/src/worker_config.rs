/// Worker runtime configuration resolved from environment variables.
#[derive(Debug, Clone)]
pub(crate) struct WorkerConfig {
    /// Base URL of the API server to poll.
    pub(crate) server_url: String,
    /// Stable worker identifier sent to claim/complete endpoints.
    pub(crate) worker_id: String,
    /// Poll interval in milliseconds for claim loop backoff.
    pub(crate) poll_ms: u64,
}

/// Resolves server URL with default fallback.
pub(crate) fn resolve_server_url(raw: Option<&str>) -> String {
    raw.unwrap_or("http://127.0.0.1:8080").to_string()
}

/// Resolves worker identifier with default fallback.
pub(crate) fn resolve_worker_id(raw: Option<&str>) -> String {
    raw.unwrap_or("worker-local").to_string()
}

/// Parses polling interval with safe default when input is invalid.
pub(crate) fn parse_poll_ms(raw: Option<&str>) -> u64 {
    raw.and_then(|v| v.parse::<u64>().ok()).unwrap_or(250)
}

/// Loads worker configuration from process environment.
pub(crate) fn load_worker_config() -> WorkerConfig {
    WorkerConfig {
        server_url: resolve_server_url(std::env::var("TARDIGRADE_SERVER_URL").ok().as_deref()),
        worker_id: resolve_worker_id(std::env::var("TARDIGRADE_WORKER_ID").ok().as_deref()),
        poll_ms: parse_poll_ms(std::env::var("TARDIGRADE_WORKER_POLL_MS").ok().as_deref()),
    }
}
