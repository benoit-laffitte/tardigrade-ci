use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use tardigrade_core::{
    BuildRecord, JobDefinition, ScmPollingConfig, ScmProvider, WebhookSecurityConfig,
};
use tokio_postgres::NoTls;
use uuid::Uuid;

use crate::Storage;
use crate::codec::{scm_provider_to_str, status_to_str};
use crate::mapping::{
    row_to_build, row_to_job, row_to_scm_polling_config, row_to_webhook_security_config,
};

/// Ordered schema migrations applied at startup for postgres-backed persistence.
const MIGRATIONS: &[(&str, &str)] = &[
    (
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
    ),
    (
        "003_init_scm_polling_configs",
        r#"
        CREATE TABLE IF NOT EXISTS scm_polling_configs (
            repository_url TEXT NOT NULL,
            provider TEXT NOT NULL,
            enabled BOOLEAN NOT NULL,
            interval_secs BIGINT NOT NULL,
            branches JSONB NOT NULL DEFAULT '[]'::jsonb,
            last_polled_at TIMESTAMPTZ NULL,
            updated_at TIMESTAMPTZ NOT NULL,
            PRIMARY KEY (repository_url, provider)
        );
        "#,
    ),
];

/// Postgres-backed implementation of the storage contract.
#[derive(Clone)]
pub struct PostgresStorage {
    client: Arc<tokio_postgres::Client>,
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

    /// Upserts SCM polling configuration in postgres.
    async fn upsert_scm_polling_config(&self, config: ScmPollingConfig) -> Result<()> {
        let branches = serde_json::to_value(&config.branches)?;
        let interval_secs = i64::try_from(config.interval_secs)
            .map_err(|_| anyhow!("interval_secs exceeds i64 range"))?;
        self.client
            .execute(
                r#"
            INSERT INTO scm_polling_configs (
                repository_url,
                provider,
                enabled,
                interval_secs,
                branches,
                last_polled_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (repository_url, provider) DO UPDATE
            SET enabled = EXCLUDED.enabled,
                interval_secs = EXCLUDED.interval_secs,
                branches = EXCLUDED.branches,
                last_polled_at = EXCLUDED.last_polled_at,
                updated_at = EXCLUDED.updated_at
            "#,
                &[
                    &config.repository_url,
                    &scm_provider_to_str(config.provider),
                    &config.enabled,
                    &interval_secs,
                    &branches,
                    &config.last_polled_at,
                    &config.updated_at,
                ],
            )
            .await?;

        Ok(())
    }

    /// Lists SCM polling configuration entries from postgres.
    async fn list_scm_polling_configs(&self) -> Result<Vec<ScmPollingConfig>> {
        let rows = self
            .client
            .query(
                r#"
            SELECT repository_url, provider, enabled, interval_secs, branches, last_polled_at, updated_at
            FROM scm_polling_configs
            "#,
                &[],
            )
            .await?;

        rows.into_iter().map(row_to_scm_polling_config).collect()
    }
}
