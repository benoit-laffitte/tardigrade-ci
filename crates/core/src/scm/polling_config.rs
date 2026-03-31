use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ScmProvider;

/// Per-repository SCM polling configuration used by background polling workers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScmPollingConfig {
    pub repository_url: String,
    pub provider: ScmProvider,
    pub enabled: bool,
    pub interval_secs: u64,
    pub branches: Vec<String>,
    pub last_polled_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}
