use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tardigrade_core::{
    BuildRecord, JobDefinition, JobStatus, ScmProvider, WebhookSecurityConfig,
};
use tokio_postgres::{NoTls, Row};
use uuid::Uuid;

/// Ordered schema migrations applied at startup for postgres-backed persistence.
const MIGRATIONS: &[(&str, &str)] = &[(
    "001_init_jobs_builds",
    r#"
        CREATE TABLE IF NOT EXISTS jobs (
            id UUID PRIMARY KEY,
            name TEXT NOT NULL,
            repository_url TEXT NOT NULL,
            pipeline_path TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL
        );

        CREATE TABLE IF NOT EXISTS builds (
            id UUID PRIMARY KEY,
            job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
            status TEXT NOT NULL,
            queued_at TIMESTAMPTZ NOT NULL,
            started_at TIMESTAMPTZ NULL,
            finished_at TIMESTAMPTZ NULL,
            logs JSONB NOT NULL DEFAULT '[]'::jsonb
        );
        "#,
),
    (
        "002_init_webhook_security_configs",
        r#"
        CREATE TABLE IF NOT EXISTS webhook_security_configs (
            repository_url TEXT NOT NULL,
            provider TEXT NOT NULL,
            secret TEXT NOT NULL,
            allowed_ips JSONB NOT NULL DEFAULT '[]'::jsonb,
            updated_at TIMESTAMPTZ NOT NULL,
            PRIMARY KEY (repository_url, provider)
        );
        "#,
    )];

#[async_trait]
pub trait Storage {
    /// Storage is the single source of truth for job/build lifecycle state.
    /// Persists or updates a job declaration.
    async fn save_job(&self, job: JobDefinition) -> Result<()>;
    /// Retrieves one job by id when present.
    async fn get_job(&self, id: Uuid) -> Result<Option<JobDefinition>>;
    /// Lists all known jobs.
    async fn list_jobs(&self) -> Result<Vec<JobDefinition>>;
    /// Persists or updates a build record.
    async fn save_build(&self, build: BuildRecord) -> Result<()>;
    /// Retrieves one build by id when present.
    async fn get_build(&self, id: Uuid) -> Result<Option<BuildRecord>>;
    /// Lists all known builds.
    async fn list_builds(&self) -> Result<Vec<BuildRecord>>;
    /// Upserts one repository-level webhook verification configuration.
    async fn upsert_webhook_security_config(&self, config: WebhookSecurityConfig) -> Result<()>;
    /// Fetches repository-level webhook verification configuration for one provider.
    async fn get_webhook_security_config(
        &self,
        repository_url: &str,
        provider: ScmProvider,
    ) -> Result<Option<WebhookSecurityConfig>>;
}

/// Postgres-backed implementation of the storage contract.
#[derive(Clone)]
pub struct PostgresStorage {
    client: Arc<tokio_postgres::Client>,
}

/// In-memory implementation used for tests and bootstrap mode.
#[derive(Clone, Default)]
pub struct InMemoryStorage {
    jobs: Arc<Mutex<HashMap<Uuid, JobDefinition>>>,
    builds: Arc<Mutex<HashMap<Uuid, BuildRecord>>>,
    webhook_security_configs: Arc<Mutex<HashMap<(String, ScmProvider), WebhookSecurityConfig>>>,
}

#[async_trait]
impl Storage for InMemoryStorage {
    /// Stores a job in process memory.
    async fn save_job(&self, job: JobDefinition) -> Result<()> {
        let mut jobs = self.jobs.lock().expect("jobs storage poisoned");
        jobs.insert(job.id, job);
        Ok(())
    }

    /// Reads a job from process memory.
    async fn get_job(&self, id: Uuid) -> Result<Option<JobDefinition>> {
        let jobs = self.jobs.lock().expect("jobs storage poisoned");
        Ok(jobs.get(&id).cloned())
    }

    /// Lists jobs from process memory.
    async fn list_jobs(&self) -> Result<Vec<JobDefinition>> {
        let jobs = self.jobs.lock().expect("jobs storage poisoned");
        Ok(jobs.values().cloned().collect())
    }

    /// Stores a build in process memory.
    async fn save_build(&self, build: BuildRecord) -> Result<()> {
        let mut builds = self.builds.lock().expect("builds storage poisoned");
        builds.insert(build.id, build);
        Ok(())
    }

    /// Reads a build from process memory.
    async fn get_build(&self, id: Uuid) -> Result<Option<BuildRecord>> {
        let builds = self.builds.lock().expect("builds storage poisoned");
        Ok(builds.get(&id).cloned())
    }

    /// Lists builds from process memory.
    async fn list_builds(&self) -> Result<Vec<BuildRecord>> {
        let builds = self.builds.lock().expect("builds storage poisoned");
        Ok(builds.values().cloned().collect())
    }

    /// Upserts repository-level webhook verification settings in process memory.
    async fn upsert_webhook_security_config(&self, config: WebhookSecurityConfig) -> Result<()> {
        let mut configs = self
            .webhook_security_configs
            .lock()
            .expect("webhook security storage poisoned");
        configs.insert(
            (config.repository_url.clone(), config.provider),
            config,
        );
        Ok(())
    }

    /// Fetches one repository-level webhook verification setting from process memory.
    async fn get_webhook_security_config(
        &self,
        repository_url: &str,
        provider: ScmProvider,
    ) -> Result<Option<WebhookSecurityConfig>> {
        let configs = self
            .webhook_security_configs
            .lock()
            .expect("webhook security storage poisoned");
        Ok(configs
            .get(&(repository_url.to_string(), provider))
            .cloned())
    }
}

impl PostgresStorage {
    /// Opens postgres connection, ensures migration table, and applies pending migrations.
    pub async fn connect(database_url: &str) -> Result<Self> {
        let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;
        tokio::spawn(async move {
            // The connection task must stay alive for all client operations.
            if let Err(error) = connection.await {
                eprintln!("postgres connection error: {error}");
            }
        });

        client
            .batch_execute(
                r#"
                CREATE TABLE IF NOT EXISTS schema_migrations (
                    version TEXT PRIMARY KEY,
                    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
                );
                "#,
            )
            .await?;

        for (version, sql) in MIGRATIONS {
            let already_applied = client
                .query_opt(
                    "SELECT version FROM schema_migrations WHERE version = $1",
                    &[version],
                )
                .await?
                .is_some();

            if already_applied {
                continue;
            }

            // Migrations are applied one-by-one and recorded only after SQL succeeds.
            client.batch_execute(sql).await?;
            client
                .execute(
                    "INSERT INTO schema_migrations (version) VALUES ($1)",
                    &[version],
                )
                .await?;
        }

        Ok(Self {
            client: Arc::new(client),
        })
    }
}

#[async_trait]
impl Storage for PostgresStorage {
    /// Upserts a job row in postgres.
    async fn save_job(&self, job: JobDefinition) -> Result<()> {
        // Upsert keeps API semantics idempotent when the same aggregate is saved multiple times.
        self.client
            .execute(
                r#"
            INSERT INTO jobs (id, name, repository_url, pipeline_path, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE
            SET name = EXCLUDED.name,
                repository_url = EXCLUDED.repository_url,
                pipeline_path = EXCLUDED.pipeline_path,
                created_at = EXCLUDED.created_at
            "#,
                &[
                    &job.id,
                    &job.name,
                    &job.repository_url,
                    &job.pipeline_path,
                    &job.created_at,
                ],
            )
            .await?;

        Ok(())
    }

    /// Fetches a single job row from postgres.
    async fn get_job(&self, id: Uuid) -> Result<Option<JobDefinition>> {
        let row = self
            .client
            .query_opt(
            "SELECT id, name, repository_url, pipeline_path, created_at FROM jobs WHERE id = $1",
            &[&id],
        )
        .await?;

        row.map(row_to_job).transpose()
    }

    /// Fetches all job rows from postgres.
    async fn list_jobs(&self) -> Result<Vec<JobDefinition>> {
        let rows = self
            .client
            .query(
                "SELECT id, name, repository_url, pipeline_path, created_at FROM jobs",
                &[],
            )
            .await?;

        rows.into_iter().map(row_to_job).collect()
    }

    /// Upserts a build row in postgres.
    async fn save_build(&self, build: BuildRecord) -> Result<()> {
        // Logs are persisted as JSON so we keep append-only event traces per build.
        let logs = serde_json::to_value(&build.logs)?;

        self.client
            .execute(
                r#"
            INSERT INTO builds (id, job_id, status, queued_at, started_at, finished_at, logs)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO UPDATE
            SET job_id = EXCLUDED.job_id,
                status = EXCLUDED.status,
                queued_at = EXCLUDED.queued_at,
                started_at = EXCLUDED.started_at,
                finished_at = EXCLUDED.finished_at,
                logs = EXCLUDED.logs
            "#,
                &[
                    &build.id,
                    &build.job_id,
                    &status_to_str(&build.status),
                    &build.queued_at,
                    &build.started_at,
                    &build.finished_at,
                    &logs,
                ],
            )
            .await?;

        Ok(())
    }

    /// Fetches a single build row from postgres.
    async fn get_build(&self, id: Uuid) -> Result<Option<BuildRecord>> {
        let row = self
            .client
            .query_opt(
            "SELECT id, job_id, status, queued_at, started_at, finished_at, logs FROM builds WHERE id = $1",
            &[&id],
        )
        .await?;

        row.map(row_to_build).transpose()
    }

    /// Fetches all build rows from postgres.
    async fn list_builds(&self) -> Result<Vec<BuildRecord>> {
        let rows = self
            .client
            .query(
                "SELECT id, job_id, status, queued_at, started_at, finished_at, logs FROM builds",
                &[],
            )
            .await?;

        rows.into_iter().map(row_to_build).collect()
    }

    /// Upserts repository-level webhook verification settings in postgres.
    async fn upsert_webhook_security_config(&self, config: WebhookSecurityConfig) -> Result<()> {
        let allowed_ips = serde_json::to_value(&config.allowed_ips)?;
        self.client
            .execute(
                r#"
            INSERT INTO webhook_security_configs (repository_url, provider, secret, allowed_ips, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (repository_url, provider) DO UPDATE
            SET secret = EXCLUDED.secret,
                allowed_ips = EXCLUDED.allowed_ips,
                updated_at = EXCLUDED.updated_at
            "#,
                &[
                    &config.repository_url,
                    &scm_provider_to_str(config.provider),
                    &config.secret,
                    &allowed_ips,
                    &config.updated_at,
                ],
            )
            .await?;

        Ok(())
    }

    /// Fetches one repository-level webhook verification setting from postgres.
    async fn get_webhook_security_config(
        &self,
        repository_url: &str,
        provider: ScmProvider,
    ) -> Result<Option<WebhookSecurityConfig>> {
        let row = self
            .client
            .query_opt(
                r#"
            SELECT repository_url, provider, secret, allowed_ips, updated_at
            FROM webhook_security_configs
            WHERE repository_url = $1 AND provider = $2
            "#,
                &[&repository_url, &scm_provider_to_str(provider)],
            )
            .await?;

        row.map(row_to_webhook_security_config).transpose()
    }
}

/// Converts a postgres row into domain job structure.
fn row_to_job(row: Row) -> Result<JobDefinition> {
    Ok(JobDefinition {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        repository_url: row.try_get("repository_url")?,
        pipeline_path: row.try_get("pipeline_path")?,
        created_at: row.try_get("created_at")?,
    })
}

/// Converts a postgres row into domain build structure.
fn row_to_build(row: Row) -> Result<BuildRecord> {
    let status_raw: String = row.try_get("status")?;
    let logs_value: Value = row.try_get("logs")?;
    let logs: Vec<String> = serde_json::from_value(logs_value)?;

    Ok(BuildRecord {
        id: row.try_get("id")?,
        job_id: row.try_get("job_id")?,
        status: parse_status(&status_raw)?,
        queued_at: row.try_get("queued_at")?,
        started_at: row.try_get::<_, Option<DateTime<Utc>>>("started_at")?,
        finished_at: row.try_get::<_, Option<DateTime<Utc>>>("finished_at")?,
        logs,
    })
}

/// Maps domain status enum to compact persisted text representation.
fn status_to_str(status: &JobStatus) -> &'static str {
    // Storage uses normalized lowercase values to stay backend-agnostic.
    match status {
        JobStatus::Pending => "pending",
        JobStatus::Running => "running",
        JobStatus::Success => "success",
        JobStatus::Failed => "failed",
        JobStatus::Canceled => "canceled",
    }
}

/// Parses persisted text representation back into domain status enum.
fn parse_status(raw: &str) -> Result<JobStatus> {
    // Reject unknown states to avoid silently corrupting runtime behavior.
    match raw {
        "pending" => Ok(JobStatus::Pending),
        "running" => Ok(JobStatus::Running),
        "success" => Ok(JobStatus::Success),
        "failed" => Ok(JobStatus::Failed),
        "canceled" => Ok(JobStatus::Canceled),
        other => Err(anyhow!("unknown job status in storage: {other}")),
    }
}

/// Maps SCM provider enum to compact persisted text representation.
fn scm_provider_to_str(provider: ScmProvider) -> &'static str {
    match provider {
        ScmProvider::Github => "github",
        ScmProvider::Gitlab => "gitlab",
    }
}

/// Parses persisted SCM provider text into enum value.
fn parse_scm_provider(raw: &str) -> Result<ScmProvider> {
    match raw {
        "github" => Ok(ScmProvider::Github),
        "gitlab" => Ok(ScmProvider::Gitlab),
        other => Err(anyhow!("unknown SCM provider in storage: {other}")),
    }
}

/// Converts a postgres row into repository-level webhook verification settings.
fn row_to_webhook_security_config(row: Row) -> Result<WebhookSecurityConfig> {
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

#[cfg(test)]
mod tests;
