use serde::Deserialize;

use super::RuntimeSection;

/// Top-level config file shape used by server bootstrap.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct ServerConfigFile {
    pub runtime: RuntimeSection,
    pub server: ServerSection,
    pub storage: StorageSection,
    pub queue: QueueSection,
    pub scm: ScmSection,
    pub security: SecuritySection,
    pub dashboard: DashboardSection,
    pub service: ServiceSection,
}

/// HTTP and service identity settings for server bootstrap.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ServerSection {
    pub service_name: String,
    pub bind: String,
}

impl Default for ServerSection {
    /// Provides safe local defaults for service name and bind address.
    fn default() -> Self {
        Self {
            service_name: "tardigrade-ci".to_string(),
            bind: "0.0.0.0:8080".to_string(),
        }
    }
}

/// Storage backend settings for API persistence.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct StorageSection {
    pub database_url: Option<String>,
}

/// Queue/scheduler backend settings.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct QueueSection {
    pub backend: Option<String>,
    pub redis_url: Option<String>,
    pub redis_prefix: String,
    pub file_path: Option<String>,
    pub scheduler_database_url: Option<String>,
    pub scheduler_namespace: String,
}

impl Default for QueueSection {
    /// Defaults queue tuning fields while keeping optional backend endpoints unset.
    fn default() -> Self {
        Self {
            backend: None,
            redis_url: None,
            redis_prefix: "tardigrade".to_string(),
            file_path: None,
            scheduler_database_url: None,
            scheduler_namespace: "tardigrade".to_string(),
        }
    }
}

/// SCM polling controls.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ScmSection {
    pub polling_enabled: bool,
    pub polling_check_secs: u64,
}

impl Default for ScmSection {
    /// Defaults SCM polling to disabled with short check interval when enabled.
    fn default() -> Self {
        Self {
            polling_enabled: false,
            polling_check_secs: 5,
        }
    }
}

/// API key security settings used for control-plane authentication.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct SecuritySection {
    pub api_key: Option<String>,
}

/// Dashboard static asset settings.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct DashboardSection {
    pub web_root: Option<String>,
}

/// API reliability settings injected into service orchestration.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ServiceSection {
    pub worker_lease_timeout_secs: u64,
    pub max_retries: u32,
    pub retry_backoff_ms: u64,
    pub webhook_dedup_ttl_secs: u64,
}

impl Default for ServiceSection {
    /// Mirrors API service defaults while allowing TOML overrides.
    fn default() -> Self {
        Self {
            worker_lease_timeout_secs: 30,
            max_retries: 2,
            retry_backoff_ms: 1000,
            webhook_dedup_ttl_secs: 3600,
        }
    }
}
