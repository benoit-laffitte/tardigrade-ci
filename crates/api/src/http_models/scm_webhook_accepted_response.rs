use serde::{Deserialize, Serialize};

/// Response payload confirming webhook ingestion acceptance.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScmWebhookAcceptedResponse {
    pub status: String,
}
