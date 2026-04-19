use anyhow::Result;
use async_trait::async_trait;
use tardigrade_core::{
    BuildRecord, JobDefinition, ScmPollingConfig, ScmProvider, WebhookSecurityConfig,
};
use uuid::Uuid;

/// Storage is the single source of truth for job/build lifecycle state.
#[async_trait]
pub trait Storage {
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

    /// Upserts one SCM polling configuration for repository/provider.
    async fn upsert_scm_polling_config(&self, config: ScmPollingConfig) -> Result<()>;

    /// Lists SCM polling configuration entries.
    async fn list_scm_polling_configs(&self) -> Result<Vec<ScmPollingConfig>>;

    /// Increments persisted retry attempt counter for one build and returns the new value.
    async fn increment_retry_attempt(&self, build_id: Uuid) -> Result<u32>;

    /// Clears persisted retry attempt counter for one build.
    async fn clear_retry_attempt(&self, build_id: Uuid) -> Result<()>;

    /// Marks one build as present in persisted dead-letter registry.
    async fn add_dead_letter_build(&self, build_id: Uuid) -> Result<()>;

    /// Removes one build from persisted dead-letter registry.
    async fn remove_dead_letter_build(&self, build_id: Uuid) -> Result<()>;

    /// Lists persisted dead-letter build identifiers.
    async fn list_dead_letter_build_ids(&self) -> Result<Vec<Uuid>>;
}
