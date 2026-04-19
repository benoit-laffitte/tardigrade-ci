use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use tardigrade_core::{
    BuildRecord, JobDefinition, ScmPollingConfig, ScmProvider, WebhookSecurityConfig,
};
use tokio_postgres::NoTls;
use uuid::Uuid;

use crate::codec::{scm_provider_to_str, status_to_str};
use crate::mapping::{
    row_to_build, row_to_job, row_to_scm_polling_config, row_to_webhook_security_config,
};
use crate::ports::{RuntimeMetricsSnapshot, ScmWebhookRejectionRecord, Storage};

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
            created_at TIMESTAMPTZ NOT NULL,
            pipeline_content TEXT NULL
        );

        CREATE TABLE IF NOT EXISTS builds (
            id UUID PRIMARY KEY,
            job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
            status TEXT NOT NULL,
            queued_at TIMESTAMPTZ NOT NULL,
            started_at TIMESTAMPTZ NULL,
            finished_at TIMESTAMPTZ NULL,
            logs JSONB NOT NULL DEFAULT '[]'::jsonb,
            pipeline_used TEXT NULL
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
    (
        "004_init_build_runtime_state",
        r#"
        CREATE TABLE IF NOT EXISTS build_retry_attempts (
            build_id UUID PRIMARY KEY REFERENCES builds(id) ON DELETE CASCADE,
            attempts INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS dead_letter_builds (
            build_id UUID PRIMARY KEY REFERENCES builds(id) ON DELETE CASCADE,
            marked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    ),
    (
        "005_init_runtime_metrics_and_webhook_rejections",
        r#"
        CREATE TABLE IF NOT EXISTS runtime_metrics (
            id SMALLINT PRIMARY KEY,
            reclaimed_total BIGINT NOT NULL,
            retry_requeued_total BIGINT NOT NULL,
            ownership_conflicts_total BIGINT NOT NULL,
            dead_letter_total BIGINT NOT NULL,
            scm_webhook_received_total BIGINT NOT NULL,
            scm_webhook_accepted_total BIGINT NOT NULL,
            scm_webhook_rejected_total BIGINT NOT NULL,
            scm_webhook_duplicate_total BIGINT NOT NULL,
            scm_trigger_enqueued_builds_total BIGINT NOT NULL,
            scm_polling_ticks_total BIGINT NOT NULL,
            scm_polling_repositories_total BIGINT NOT NULL,
            scm_polling_enqueued_builds_total BIGINT NOT NULL
        );

        INSERT INTO runtime_metrics (
            id,
            reclaimed_total,
            retry_requeued_total,
            ownership_conflicts_total,
            dead_letter_total,
            scm_webhook_received_total,
            scm_webhook_accepted_total,
            scm_webhook_rejected_total,
            scm_webhook_duplicate_total,
            scm_trigger_enqueued_builds_total,
            scm_polling_ticks_total,
            scm_polling_repositories_total,
            scm_polling_enqueued_builds_total
        )
        VALUES (1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
        ON CONFLICT (id) DO NOTHING;

        CREATE TABLE IF NOT EXISTS scm_webhook_rejections (
            id BIGSERIAL PRIMARY KEY,
            reason_code TEXT NOT NULL,
            provider TEXT NULL,
            repository_url TEXT NULL,
            at TIMESTAMPTZ NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_scm_webhook_rejections_at_desc
            ON scm_webhook_rejections (at DESC, id DESC);
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
            INSERT INTO jobs (id, name, repository_url, pipeline_path, created_at, pipeline_content)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE
            SET name = EXCLUDED.name,
                repository_url = EXCLUDED.repository_url,
                pipeline_path = EXCLUDED.pipeline_path,
                created_at = EXCLUDED.created_at,
                pipeline_content = EXCLUDED.pipeline_content
            "#,
                &[
                    &job.id,
                    &job.name,
                    &job.repository_url,
                    &job.pipeline_path,
                    &job.created_at,
                    &job.pipeline_content,
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
                "SELECT id, name, repository_url, pipeline_path, created_at, pipeline_content FROM jobs WHERE id = $1",
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
                "SELECT id, name, repository_url, pipeline_path, created_at, pipeline_content FROM jobs",
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
            INSERT INTO builds (id, job_id, status, queued_at, started_at, finished_at, logs, pipeline_used)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE
            SET job_id = EXCLUDED.job_id,
                status = EXCLUDED.status,
                queued_at = EXCLUDED.queued_at,
                started_at = EXCLUDED.started_at,
                finished_at = EXCLUDED.finished_at,
                logs = EXCLUDED.logs,
                pipeline_used = EXCLUDED.pipeline_used
            "#,
                &[
                    &build.id,
                    &build.job_id,
                    &status_to_str(&build.status),
                    &build.queued_at,
                    &build.started_at,
                    &build.finished_at,
                    &logs,
                    &build.pipeline_used,
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
                "SELECT id, job_id, status, queued_at, started_at, finished_at, logs, pipeline_used FROM builds WHERE id = $1",
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
                "SELECT id, job_id, status, queued_at, started_at, finished_at, logs, pipeline_used FROM builds",
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

    /// Increments persisted retry attempt counter for one build row in postgres.
    async fn increment_retry_attempt(&self, build_id: Uuid) -> Result<u32> {
        let row = self
            .client
            .query_one(
                r#"
            INSERT INTO build_retry_attempts (build_id, attempts)
            VALUES ($1, 1)
            ON CONFLICT (build_id) DO UPDATE
            SET attempts = build_retry_attempts.attempts + 1
            RETURNING attempts
            "#,
                &[&build_id],
            )
            .await?;

        let attempts: i32 = row.get("attempts");
        let attempts = u32::try_from(attempts).map_err(|_| anyhow!("invalid attempts value"))?;
        Ok(attempts)
    }

    /// Clears persisted retry attempt counter for one build row in postgres.
    async fn clear_retry_attempt(&self, build_id: Uuid) -> Result<()> {
        self.client
            .execute(
                "DELETE FROM build_retry_attempts WHERE build_id = $1",
                &[&build_id],
            )
            .await?;
        Ok(())
    }

    /// Adds one build to persisted dead-letter registry in postgres.
    async fn add_dead_letter_build(&self, build_id: Uuid) -> Result<()> {
        self.client
            .execute(
                r#"
            INSERT INTO dead_letter_builds (build_id)
            VALUES ($1)
            ON CONFLICT (build_id) DO NOTHING
            "#,
                &[&build_id],
            )
            .await?;
        Ok(())
    }

    /// Removes one build from persisted dead-letter registry in postgres.
    async fn remove_dead_letter_build(&self, build_id: Uuid) -> Result<()> {
        self.client
            .execute(
                "DELETE FROM dead_letter_builds WHERE build_id = $1",
                &[&build_id],
            )
            .await?;
        Ok(())
    }

    /// Lists persisted dead-letter build identifiers from postgres.
    async fn list_dead_letter_build_ids(&self) -> Result<Vec<Uuid>> {
        let rows = self
            .client
            .query(
                r#"
            SELECT build_id
            FROM dead_letter_builds
            ORDER BY marked_at DESC
            "#,
                &[],
            )
            .await?;

        Ok(rows.into_iter().map(|row| row.get("build_id")).collect())
    }

    /// Persists runtime metrics snapshot in postgres singleton row.
    async fn save_runtime_metrics(&self, metrics: RuntimeMetricsSnapshot) -> Result<()> {
        self.client
            .execute(
                r#"
            INSERT INTO runtime_metrics (
                id,
                reclaimed_total,
                retry_requeued_total,
                ownership_conflicts_total,
                dead_letter_total,
                scm_webhook_received_total,
                scm_webhook_accepted_total,
                scm_webhook_rejected_total,
                scm_webhook_duplicate_total,
                scm_trigger_enqueued_builds_total,
                scm_polling_ticks_total,
                scm_polling_repositories_total,
                scm_polling_enqueued_builds_total
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (id) DO UPDATE
            SET reclaimed_total = EXCLUDED.reclaimed_total,
                retry_requeued_total = EXCLUDED.retry_requeued_total,
                ownership_conflicts_total = EXCLUDED.ownership_conflicts_total,
                dead_letter_total = EXCLUDED.dead_letter_total,
                scm_webhook_received_total = EXCLUDED.scm_webhook_received_total,
                scm_webhook_accepted_total = EXCLUDED.scm_webhook_accepted_total,
                scm_webhook_rejected_total = EXCLUDED.scm_webhook_rejected_total,
                scm_webhook_duplicate_total = EXCLUDED.scm_webhook_duplicate_total,
                scm_trigger_enqueued_builds_total = EXCLUDED.scm_trigger_enqueued_builds_total,
                scm_polling_ticks_total = EXCLUDED.scm_polling_ticks_total,
                scm_polling_repositories_total = EXCLUDED.scm_polling_repositories_total,
                scm_polling_enqueued_builds_total = EXCLUDED.scm_polling_enqueued_builds_total
            "#,
                &[
                    &1_i16,
                    &i64::try_from(metrics.reclaimed_total)
                        .map_err(|_| anyhow!("reclaimed_total exceeds i64 range"))?,
                    &i64::try_from(metrics.retry_requeued_total)
                        .map_err(|_| anyhow!("retry_requeued_total exceeds i64 range"))?,
                    &i64::try_from(metrics.ownership_conflicts_total)
                        .map_err(|_| anyhow!("ownership_conflicts_total exceeds i64 range"))?,
                    &i64::try_from(metrics.dead_letter_total)
                        .map_err(|_| anyhow!("dead_letter_total exceeds i64 range"))?,
                    &i64::try_from(metrics.scm_webhook_received_total)
                        .map_err(|_| anyhow!("scm_webhook_received_total exceeds i64 range"))?,
                    &i64::try_from(metrics.scm_webhook_accepted_total)
                        .map_err(|_| anyhow!("scm_webhook_accepted_total exceeds i64 range"))?,
                    &i64::try_from(metrics.scm_webhook_rejected_total)
                        .map_err(|_| anyhow!("scm_webhook_rejected_total exceeds i64 range"))?,
                    &i64::try_from(metrics.scm_webhook_duplicate_total)
                        .map_err(|_| anyhow!("scm_webhook_duplicate_total exceeds i64 range"))?,
                    &i64::try_from(metrics.scm_trigger_enqueued_builds_total).map_err(|_| {
                        anyhow!("scm_trigger_enqueued_builds_total exceeds i64 range")
                    })?,
                    &i64::try_from(metrics.scm_polling_ticks_total)
                        .map_err(|_| anyhow!("scm_polling_ticks_total exceeds i64 range"))?,
                    &i64::try_from(metrics.scm_polling_repositories_total)
                        .map_err(|_| anyhow!("scm_polling_repositories_total exceeds i64 range"))?,
                    &i64::try_from(metrics.scm_polling_enqueued_builds_total).map_err(|_| {
                        anyhow!("scm_polling_enqueued_builds_total exceeds i64 range")
                    })?,
                ],
            )
            .await?;
        Ok(())
    }

    /// Loads persisted runtime metrics snapshot from postgres singleton row.
    async fn load_runtime_metrics(&self) -> Result<RuntimeMetricsSnapshot> {
        let row = self
            .client
            .query_opt(
                r#"
            SELECT
                reclaimed_total,
                retry_requeued_total,
                ownership_conflicts_total,
                dead_letter_total,
                scm_webhook_received_total,
                scm_webhook_accepted_total,
                scm_webhook_rejected_total,
                scm_webhook_duplicate_total,
                scm_trigger_enqueued_builds_total,
                scm_polling_ticks_total,
                scm_polling_repositories_total,
                scm_polling_enqueued_builds_total
            FROM runtime_metrics
            WHERE id = 1
            "#,
                &[],
            )
            .await?;

        let Some(row) = row else {
            return Ok(RuntimeMetricsSnapshot::default());
        };

        Ok(RuntimeMetricsSnapshot {
            reclaimed_total: u64::try_from(row.get::<_, i64>("reclaimed_total"))
                .map_err(|_| anyhow!("invalid reclaimed_total"))?,
            retry_requeued_total: u64::try_from(row.get::<_, i64>("retry_requeued_total"))
                .map_err(|_| anyhow!("invalid retry_requeued_total"))?,
            ownership_conflicts_total: u64::try_from(
                row.get::<_, i64>("ownership_conflicts_total"),
            )
            .map_err(|_| anyhow!("invalid ownership_conflicts_total"))?,
            dead_letter_total: u64::try_from(row.get::<_, i64>("dead_letter_total"))
                .map_err(|_| anyhow!("invalid dead_letter_total"))?,
            scm_webhook_received_total: u64::try_from(
                row.get::<_, i64>("scm_webhook_received_total"),
            )
            .map_err(|_| anyhow!("invalid scm_webhook_received_total"))?,
            scm_webhook_accepted_total: u64::try_from(
                row.get::<_, i64>("scm_webhook_accepted_total"),
            )
            .map_err(|_| anyhow!("invalid scm_webhook_accepted_total"))?,
            scm_webhook_rejected_total: u64::try_from(
                row.get::<_, i64>("scm_webhook_rejected_total"),
            )
            .map_err(|_| anyhow!("invalid scm_webhook_rejected_total"))?,
            scm_webhook_duplicate_total: u64::try_from(
                row.get::<_, i64>("scm_webhook_duplicate_total"),
            )
            .map_err(|_| anyhow!("invalid scm_webhook_duplicate_total"))?,
            scm_trigger_enqueued_builds_total: u64::try_from(
                row.get::<_, i64>("scm_trigger_enqueued_builds_total"),
            )
            .map_err(|_| anyhow!("invalid scm_trigger_enqueued_builds_total"))?,
            scm_polling_ticks_total: u64::try_from(row.get::<_, i64>("scm_polling_ticks_total"))
                .map_err(|_| anyhow!("invalid scm_polling_ticks_total"))?,
            scm_polling_repositories_total: u64::try_from(
                row.get::<_, i64>("scm_polling_repositories_total"),
            )
            .map_err(|_| anyhow!("invalid scm_polling_repositories_total"))?,
            scm_polling_enqueued_builds_total: u64::try_from(
                row.get::<_, i64>("scm_polling_enqueued_builds_total"),
            )
            .map_err(|_| anyhow!("invalid scm_polling_enqueued_builds_total"))?,
        })
    }

    /// Appends one webhook rejection diagnostic row and prunes oldest rows above max_entries.
    async fn append_scm_webhook_rejection(
        &self,
        entry: ScmWebhookRejectionRecord,
        max_entries: usize,
    ) -> Result<()> {
        self.client
            .execute(
                r#"
            INSERT INTO scm_webhook_rejections (reason_code, provider, repository_url, at)
            VALUES ($1, $2, $3, $4)
            "#,
                &[
                    &entry.reason_code,
                    &entry.provider,
                    &entry.repository_url,
                    &entry.at,
                ],
            )
            .await?;

        let max_entries =
            i64::try_from(max_entries).map_err(|_| anyhow!("max_entries exceeds i64 range"))?;
        self.client
            .execute(
                r#"
            DELETE FROM scm_webhook_rejections
            WHERE id IN (
                SELECT id
                FROM scm_webhook_rejections
                ORDER BY at DESC, id DESC
                OFFSET $1
            )
            "#,
                &[&max_entries],
            )
            .await?;

        Ok(())
    }

    /// Lists recent webhook rejection diagnostics from postgres in reverse chronological order.
    async fn list_scm_webhook_rejections(
        &self,
        limit: usize,
    ) -> Result<Vec<ScmWebhookRejectionRecord>> {
        let limit = i64::try_from(limit).map_err(|_| anyhow!("limit exceeds i64 range"))?;
        let rows = self
            .client
            .query(
                r#"
            SELECT reason_code, provider, repository_url, at
            FROM scm_webhook_rejections
            ORDER BY at DESC, id DESC
            LIMIT $1
            "#,
                &[&limit],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| ScmWebhookRejectionRecord {
                reason_code: row.get("reason_code"),
                provider: row.get("provider"),
                repository_url: row.get("repository_url"),
                at: row.get("at"),
            })
            .collect())
    }
}
