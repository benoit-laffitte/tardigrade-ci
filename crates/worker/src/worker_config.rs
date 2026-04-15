/// Worker runtime configuration resolved from environment variables.
#[derive(Debug, Clone)]
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

/// Parses a boolean environment value with tolerant true/false aliases.
pub(crate) fn parse_bool(raw: Option<&str>, default: bool) -> bool {
    match raw.map(str::trim).map(str::to_ascii_lowercase) {
        Some(value) if matches!(value.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(value) if matches!(value.as_str(), "0" | "false" | "no" | "off") => false,
        Some(_) => default,
        None => default,
    }
}

/// Parses an unsigned integer environment value with fallback default.
pub(crate) fn parse_u64(raw: Option<&str>, default: u64) -> u64 {
    raw.and_then(|v| v.parse::<u64>().ok()).unwrap_or(default)
}

/// Parses a usize environment value with fallback default.
pub(crate) fn parse_usize(raw: Option<&str>, default: usize) -> usize {
    raw.and_then(|v| v.parse::<usize>().ok()).unwrap_or(default)
}

/// Loads worker configuration from process environment.
pub(crate) fn load_worker_config() -> WorkerConfig {
    WorkerConfig {
        server_url: resolve_server_url(std::env::var("TARDIGRADE_SERVER_URL").ok().as_deref()),
        worker_id: resolve_worker_id(std::env::var("TARDIGRADE_WORKER_ID").ok().as_deref()),
        poll_ms: parse_poll_ms(std::env::var("TARDIGRADE_WORKER_POLL_MS").ok().as_deref()),
        http2_enabled: parse_bool(
            std::env::var("TARDIGRADE_WORKER_HTTP2_ENABLED")
                .ok()
                .as_deref(),
            true,
        ),
        http2_prior_knowledge: parse_bool(
            std::env::var("TARDIGRADE_WORKER_HTTP2_PRIOR_KNOWLEDGE")
                .ok()
                .as_deref(),
            false,
        ),
        request_timeout_secs: parse_u64(
            std::env::var("TARDIGRADE_WORKER_REQUEST_TIMEOUT_SECS")
                .ok()
                .as_deref(),
            30,
        ),
        pool_idle_timeout_secs: parse_u64(
            std::env::var("TARDIGRADE_WORKER_POOL_IDLE_TIMEOUT_SECS")
                .ok()
                .as_deref(),
            90,
        ),
        pool_max_idle_per_host: parse_usize(
            std::env::var("TARDIGRADE_WORKER_POOL_MAX_IDLE_PER_HOST")
                .ok()
                .as_deref(),
            32,
        ),
        http2_keep_alive_secs: parse_u64(
            std::env::var("TARDIGRADE_WORKER_HTTP2_KEEP_ALIVE_SECS")
                .ok()
                .as_deref(),
            30,
        ),
    }
}

#[cfg(test)]
mod tests;
