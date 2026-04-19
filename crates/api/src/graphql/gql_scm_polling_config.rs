use async_graphql::SimpleObject;
use tardigrade_core::ScmProvider;
use chrono::{DateTime, Utc};

/// GraphQL projection for one SCM polling config entry.
#[derive(Clone, SimpleObject)]
#[graphql(rename_fields = "snake_case")]
pub(crate) struct GqlScmPollingConfig {
    pub(crate) repository_url: String,
    pub(crate) provider: String,
    pub(crate) enabled: bool,
    pub(crate) interval_secs: i32,
    pub(crate) branches: Vec<String>,
    pub(crate) last_polled_at: Option<DateTime<Utc>>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl From<tardigrade_core::ScmPollingConfig> for GqlScmPollingConfig {
    fn from(value: tardigrade_core::ScmPollingConfig) -> Self {
        Self {
            repository_url: value.repository_url,
            provider: value.provider.to_string(),
            enabled: value.enabled,
            interval_secs: value.interval_secs as i32,
            branches: value.branches,
            last_polled_at: value.last_polled_at,
            updated_at: value.updated_at,
        }
    }
}
