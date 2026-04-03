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
}
