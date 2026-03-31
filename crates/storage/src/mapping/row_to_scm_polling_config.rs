use anyhow::{Result, anyhow};
use serde_json::Value;
use tardigrade_core::ScmPollingConfig;
use tokio_postgres::Row;

use crate::codec::parse_scm_provider;

/// Converts a postgres row into SCM polling configuration model.
pub(crate) fn row_to_scm_polling_config(row: Row) -> Result<ScmPollingConfig> {
    let provider_raw: String = row.try_get("provider")?;
    let branches_value: Value = row.try_get("branches")?;
    let branches: Vec<String> = serde_json::from_value(branches_value)?;
    let interval_secs_i64: i64 = row.try_get("interval_secs")?;
    let interval_secs = u64::try_from(interval_secs_i64)
        .map_err(|_| anyhow!("negative interval_secs in storage: {interval_secs_i64}"))?;

    Ok(ScmPollingConfig {
        repository_url: row.try_get("repository_url")?,
        provider: parse_scm_provider(&provider_raw)?,
        enabled: row.try_get("enabled")?,
        interval_secs,
        branches,
        last_polled_at: row.try_get("last_polled_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}
