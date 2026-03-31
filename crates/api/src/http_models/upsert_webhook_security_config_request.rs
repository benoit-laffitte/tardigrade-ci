use serde::Deserialize;
use tardigrade_core::ScmProvider;

/// Request payload used to register webhook verification settings for one repository.
#[derive(Debug, Deserialize)]
pub struct UpsertWebhookSecurityConfigRequest {
    pub repository_url: String,
    pub provider: ScmProvider,
    pub secret: String,
    #[serde(default)]
    pub allowed_ips: Vec<String>,
}
