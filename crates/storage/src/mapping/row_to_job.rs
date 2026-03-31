use anyhow::Result;
use tardigrade_core::JobDefinition;
use tokio_postgres::Row;

/// Converts a postgres row into domain job structure.
pub(crate) fn row_to_job(row: Row) -> Result<JobDefinition> {
    Ok(JobDefinition {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        repository_url: row.try_get("repository_url")?,
        pipeline_path: row.try_get("pipeline_path")?,
        created_at: row.try_get("created_at")?,
    })
}
