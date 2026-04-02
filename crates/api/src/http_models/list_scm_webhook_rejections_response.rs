use serde::{Deserialize, Serialize};

use super::ScmWebhookRejectionEntry;

/// Response payload listing recent SCM webhook rejection diagnostics.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListScmWebhookRejectionsResponse {
    pub rejections: Vec<ScmWebhookRejectionEntry>,
}
