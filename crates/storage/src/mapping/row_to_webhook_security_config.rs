use anyhow::Result;
use serde_json::Value;
use tardigrade_core::WebhookSecurityConfig;
use tokio_postgres::Row;

use crate::codec::parse_scm_provider;

/// Converts a postgres row into repository-level webhook verification settings.
pub(crate) fn row_to_webhook_security_config(row: Row) -> Result<WebhookSecurityConfig> {
    let provider_raw: String = row.try_get("provider")?;
    let allowed_ips_value: Value = row.try_get("allowed_ips")?;
    let allowed_ips: Vec<String> = serde_json::from_value(allowed_ips_value)?;

    Ok(WebhookSecurityConfig {
        repository_url: row.try_get("repository_url")?,
        provider: parse_scm_provider(&provider_raw)?,
        secret: row.try_get("secret")?,
        allowed_ips,
        updated_at: row.try_get("updated_at")?,
    })
}
