use serde::Deserialize;
use tardigrade_core::ScmProvider;

/// Request payload used to upsert SCM polling settings for one repository/provider.
#[derive(Debug, Deserialize)]
pub struct UpsertScmPollingConfigRequest {
    pub repository_url: String,
    pub provider: ScmProvider,
    pub enabled: bool,
    pub interval_secs: u64,
    #[serde(default)]
    pub branches: Vec<String>,
}
