use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ScmProvider;

/// Per-repository webhook security settings persisted for SCM trigger validation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebhookSecurityConfig {
    pub repository_url: String,
    pub provider: ScmProvider,
    pub secret: String,
    pub allowed_ips: Vec<String>,
    pub updated_at: DateTime<Utc>,
}
