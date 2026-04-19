use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use tardigrade_core::BuildRecord;
use tokio_postgres::Row;

use crate::codec::parse_status;

/// Converts a postgres row into domain build structure.
pub(crate) fn row_to_build(row: Row) -> Result<BuildRecord> {
    let status_raw: String = row.try_get("status")?;
    let logs_value: Value = row.try_get("logs")?;
    let logs: Vec<String> = serde_json::from_value(logs_value)?;

    Ok(BuildRecord {
        id: row.try_get("id")?,
        job_id: row.try_get("job_id")?,
        status: parse_status(&status_raw)?,
        queued_at: row.try_get("queued_at")?,
        started_at: row.try_get("started_at")?,
        finished_at: row.try_get("finished_at")?,
        logs,
        pipeline_used: row.try_get("pipeline_used").ok(),
    })
}
