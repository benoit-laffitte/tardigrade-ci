use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tardigrade_core::{BuildRecord, JobDefinition, ScmPollingConfig, ScmProvider, WebhookSecurityConfig};
use uuid::Uuid;

use crate::Storage;

/// In-memory implementation used for tests and bootstrap mode.
#[derive(Clone, Default)]
pub struct InMemoryStorage {
    jobs: Arc<Mutex<HashMap<Uuid, JobDefinition>>>,
    builds: Arc<Mutex<HashMap<Uuid, BuildRecord>>>,
    webhook_security_configs: Arc<Mutex<HashMap<(String, ScmProvider), WebhookSecurityConfig>>>,
    scm_polling_configs: Arc<Mutex<HashMap<(String, ScmProvider), ScmPollingConfig>>>,
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
        configs.insert((config.repository_url.clone(), config.provider), config);
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
        Ok(configs.get(&(repository_url.to_string(), provider)).cloned())
    }

    /// Upserts SCM polling configuration in process memory.
    async fn upsert_scm_polling_config(&self, config: ScmPollingConfig) -> Result<()> {
        let mut configs = self
            .scm_polling_configs
            .lock()
            .expect("scm polling storage poisoned");
        configs.insert((config.repository_url.clone(), config.provider), config);
        Ok(())
    }

    /// Lists SCM polling configuration entries from process memory.
    async fn list_scm_polling_configs(&self) -> Result<Vec<ScmPollingConfig>> {
        let configs = self
            .scm_polling_configs
            .lock()
            .expect("scm polling storage poisoned");
        Ok(configs.values().cloned().collect())
    }
}
