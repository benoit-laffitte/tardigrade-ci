use serde::{Deserialize, Serialize};

/// One rejected SCM webhook diagnostic entry with reason and context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScmWebhookRejectionEntry {
    pub reason_code: String,
    pub provider: Option<String>,
    pub repository_url: Option<String>,
    pub at: String,
}
