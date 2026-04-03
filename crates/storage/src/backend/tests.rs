use super::InMemoryStorage;
use crate::Storage;
use chrono::Utc;
use tardigrade_core::{
    BuildRecord, JobDefinition, JobStatus, ScmPollingConfig, ScmProvider, WebhookSecurityConfig,
};

/// Verifies in-memory storage roundtrip for one job and one build.
#[tokio::test]
async fn in_memory_storage_roundtrip_job_and_build() {
    let storage = InMemoryStorage::default();
    let job = JobDefinition::new(
        "build-api".to_string(),
        "https://example.com/repo.git".to_string(),
        "pipeline.yml".to_string(),
    );
    let mut build = BuildRecord::queued(job.id);
    build.append_log("queued");

    storage
        .save_job(job.clone())
        .await
        .expect("save job should succeed");
    storage
        .save_build(build.clone())
        .await
        .expect("save build should succeed");

    let stored_job = storage
        .get_job(job.id)
        .await
        .expect("get job should succeed")
        .expect("job should exist");
    let stored_build = storage
        .get_build(build.id)
        .await
        .expect("get build should succeed")
        .expect("build should exist");

    assert_eq!(stored_job.id, job.id);
    assert_eq!(stored_job.name, "build-api");
    assert_eq!(stored_build.id, build.id);
    assert_eq!(stored_build.status, JobStatus::Pending);
    assert_eq!(stored_build.logs, vec!["queued".to_string()]);

    let listed_jobs = storage.list_jobs().await.expect("list jobs should succeed");
    let listed_builds = storage
        .list_builds()
        .await
        .expect("list builds should succeed");
    assert_eq!(listed_jobs.len(), 1);
    assert_eq!(listed_builds.len(), 1);
}

/// Verifies in-memory storage behavior for empty lists and missing ids.
#[tokio::test]
async fn in_memory_storage_handles_empty_and_missing_records() {
    let storage = InMemoryStorage::default();
    let missing = uuid::Uuid::new_v4();

    let jobs = storage.list_jobs().await.expect("list jobs should succeed");
    let builds = storage
        .list_builds()
        .await
        .expect("list builds should succeed");
    assert!(jobs.is_empty());
    assert!(builds.is_empty());

    assert!(
        storage
            .get_job(missing)
            .await
            .expect("get job should succeed")
            .is_none()
    );
    assert!(
        storage
            .get_build(missing)
            .await
            .expect("get build should succeed")
            .is_none()
    );
}

/// Verifies saving a job with the same id updates existing entry.
#[tokio::test]
async fn in_memory_storage_overwrites_existing_job_by_id() {
    let storage = InMemoryStorage::default();
    let mut original = JobDefinition::new(
        "build-api".to_string(),
        "https://example.com/repo.git".to_string(),
        "pipeline.yml".to_string(),
    );
    let mut updated = original.clone();
    updated.name = "build-api-updated".to_string();

    storage
        .save_job(original.clone())
        .await
        .expect("save original should succeed");
    storage
        .save_job(updated.clone())
        .await
        .expect("save updated should succeed");

    original = storage
        .get_job(original.id)
        .await
        .expect("get job should succeed")
        .expect("job should exist");
    assert_eq!(original.name, "build-api-updated");
}

/// Verifies in-memory webhook security config roundtrip.
#[tokio::test]
async fn in_memory_storage_roundtrip_webhook_security_config() {
    let storage = InMemoryStorage::default();
    let config = WebhookSecurityConfig {
        repository_url: "https://example.com/repo.git".to_string(),
        provider: ScmProvider::Github,
        secret: "secret-1".to_string(),
        allowed_ips: vec!["203.0.113.10".to_string()],
        updated_at: Utc::now(),
    };

    storage
        .upsert_webhook_security_config(config.clone())
        .await
        .expect("save webhook config should succeed");

    let stored = storage
        .get_webhook_security_config(&config.repository_url, config.provider)
        .await
        .expect("get webhook config should succeed")
        .expect("webhook config should exist");

    assert_eq!(stored.repository_url, config.repository_url);
    assert_eq!(stored.provider, config.provider);
    assert_eq!(stored.secret, "secret-1");
    assert_eq!(stored.allowed_ips, vec!["203.0.113.10".to_string()]);
}

/// Verifies in-memory SCM polling config roundtrip.
#[tokio::test]
async fn in_memory_storage_roundtrip_scm_polling_config() {
    let storage = InMemoryStorage::default();
    let config = ScmPollingConfig {
        repository_url: "https://example.com/repo.git".to_string(),
        provider: ScmProvider::Github,
        enabled: true,
        interval_secs: 30,
        branches: vec!["main".to_string(), "develop".to_string()],
        last_polled_at: None,
        updated_at: Utc::now(),
    };

    storage
        .upsert_scm_polling_config(config.clone())
        .await
        .expect("save scm polling config should succeed");

    let listed = storage
        .list_scm_polling_configs()
        .await
        .expect("list scm polling configs should succeed");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].repository_url, config.repository_url);
    assert_eq!(listed[0].provider, config.provider);
    assert_eq!(listed[0].interval_secs, 30);
    assert_eq!(listed[0].branches.len(), 2);
}
