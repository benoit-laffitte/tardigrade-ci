use super::{InMemoryStorage, PostgresStorage};
use crate::ports::{RuntimeMetricsSnapshot, ScmWebhookRejectionRecord, Storage};
use chrono::{Duration as ChronoDuration, Utc};
use tardigrade_core::{
    BuildRecord, JobDefinition, JobStatus, ScmPollingConfig, ScmProvider, WebhookSecurityConfig,
};
use uuid::Uuid;

/// Verifies in-memory storage roundtrip for one job and one build.
#[tokio::test]
async fn in_memory_storage_roundtrip_job_and_build() {
    let storage = InMemoryStorage::default();
    let job = JobDefinition::new(
        "build-api".to_string(),
        "https://example.com/repo.git".to_string(),
        "pipeline.yml".to_string(),
        None,
    );
    let mut build = BuildRecord::queued(job.id, None);
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
        None,
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

/// Verifies in-memory storage persists retry counters and dead-letter registry operations.
#[tokio::test]
async fn in_memory_storage_persists_retry_and_dead_letter_runtime_state() {
    let storage = InMemoryStorage::default();
    let build_id = Uuid::new_v4();

    let attempt_1 = storage
        .increment_retry_attempt(build_id)
        .await
        .expect("increment retry attempt should succeed");
    let attempt_2 = storage
        .increment_retry_attempt(build_id)
        .await
        .expect("increment retry attempt should succeed");
    assert_eq!(attempt_1, 1);
    assert_eq!(attempt_2, 2);

    storage
        .add_dead_letter_build(build_id)
        .await
        .expect("add dead letter build should succeed");
    let dead_letter_ids = storage
        .list_dead_letter_build_ids()
        .await
        .expect("list dead letter builds should succeed");
    assert_eq!(dead_letter_ids, vec![build_id]);

    storage
        .clear_retry_attempt(build_id)
        .await
        .expect("clear retry attempt should succeed");
    let reset_attempt = storage
        .increment_retry_attempt(build_id)
        .await
        .expect("increment retry attempt should succeed after clear");
    assert_eq!(reset_attempt, 1);

    storage
        .remove_dead_letter_build(build_id)
        .await
        .expect("remove dead letter build should succeed");
    let dead_letter_ids = storage
        .list_dead_letter_build_ids()
        .await
        .expect("list dead letter builds should succeed");
    assert!(dead_letter_ids.is_empty());
}

/// Verifies in-memory storage persists runtime metrics snapshot and webhook rejection history.
#[tokio::test]
async fn in_memory_storage_persists_runtime_metrics_and_webhook_rejections() {
    let storage = InMemoryStorage::default();
    let now = Utc::now();
    let metrics = RuntimeMetricsSnapshot {
        reclaimed_total: 3,
        retry_requeued_total: 5,
        ownership_conflicts_total: 7,
        dead_letter_total: 11,
        scm_webhook_received_total: 13,
        scm_webhook_accepted_total: 17,
        scm_webhook_rejected_total: 19,
        scm_webhook_duplicate_total: 23,
        scm_trigger_enqueued_builds_total: 29,
        scm_polling_ticks_total: 31,
        scm_polling_repositories_total: 37,
        scm_polling_enqueued_builds_total: 41,
    };

    storage
        .save_runtime_metrics(metrics.clone())
        .await
        .expect("save runtime metrics should succeed");
    let loaded = storage
        .load_runtime_metrics()
        .await
        .expect("load runtime metrics should succeed");
    assert_eq!(loaded, metrics);

    storage
        .append_scm_webhook_rejection(
            ScmWebhookRejectionRecord {
                reason_code: "forbidden".to_string(),
                provider: Some("github".to_string()),
                repository_url: Some("https://example.com/repo.git".to_string()),
                at: now,
            },
            2,
        )
        .await
        .expect("append webhook rejection should succeed");
    storage
        .append_scm_webhook_rejection(
            ScmWebhookRejectionRecord {
                reason_code: "invalid_signature".to_string(),
                provider: Some("github".to_string()),
                repository_url: Some("https://example.com/repo.git".to_string()),
                at: now + ChronoDuration::seconds(1),
            },
            2,
        )
        .await
        .expect("append webhook rejection should succeed");
    storage
        .append_scm_webhook_rejection(
            ScmWebhookRejectionRecord {
                reason_code: "duplicate".to_string(),
                provider: Some("github".to_string()),
                repository_url: Some("https://example.com/repo.git".to_string()),
                at: now + ChronoDuration::seconds(2),
            },
            2,
        )
        .await
        .expect("append webhook rejection should prune oldest rows");

    let rows = storage
        .list_scm_webhook_rejections(10)
        .await
        .expect("list webhook rejections should succeed");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].reason_code, "duplicate");
    assert_eq!(rows[1].reason_code, "invalid_signature");
}

/// Verifies postgres storage keeps retry and dead-letter runtime state across state recreation.
#[tokio::test]
async fn postgres_storage_persists_retry_and_dead_letter_across_state_recreation() {
    let database_url = match std::env::var("TARDIGRADE_TEST_DATABASE_URL") {
        Ok(value) => value,
        Err(_) => return,
    };

    let build_id = Uuid::new_v4();
    let job = JobDefinition::new(
        format!("coreci05-job-{build_id}"),
        "https://example.com/repo.git".to_string(),
        "pipeline.yml".to_string(),
        None,
    );
    let build = BuildRecord::queued(job.id, None);

    let storage_a = PostgresStorage::connect(&database_url)
        .await
        .expect("connect postgres storage");
    storage_a
        .save_job(job.clone())
        .await
        .expect("save job should succeed");
    storage_a
        .save_build(BuildRecord {
            id: build_id,
            ..build.clone()
        })
        .await
        .expect("save build should succeed");

    let attempt_1 = storage_a
        .increment_retry_attempt(build_id)
        .await
        .expect("increment retry attempt should succeed");
    let attempt_2 = storage_a
        .increment_retry_attempt(build_id)
        .await
        .expect("increment retry attempt should succeed");
    assert_eq!(attempt_1, 1);
    assert_eq!(attempt_2, 2);

    storage_a
        .add_dead_letter_build(build_id)
        .await
        .expect("add dead letter build should succeed");

    let storage_b = PostgresStorage::connect(&database_url)
        .await
        .expect("reconnect postgres storage");
    let attempt_3 = storage_b
        .increment_retry_attempt(build_id)
        .await
        .expect("increment retry attempt should continue persisted value");
    assert_eq!(attempt_3, 3);

    let dead_letter_ids = storage_b
        .list_dead_letter_build_ids()
        .await
        .expect("list dead letter builds should succeed");
    assert!(dead_letter_ids.contains(&build_id));

    storage_b
        .clear_retry_attempt(build_id)
        .await
        .expect("clear retry attempt should succeed");
    storage_b
        .remove_dead_letter_build(build_id)
        .await
        .expect("remove dead letter build should succeed");
}

/// Verifies postgres storage persists runtime metrics and webhook rejection history across reconnect.
#[tokio::test]
async fn postgres_storage_persists_metrics_and_webhook_rejections_across_state_recreation() {
    let database_url = match std::env::var("TARDIGRADE_TEST_DATABASE_URL") {
        Ok(value) => value,
        Err(_) => return,
    };

    let now = Utc::now();
    let storage_a = PostgresStorage::connect(&database_url)
        .await
        .expect("connect postgres storage");
    let metrics = RuntimeMetricsSnapshot {
        reclaimed_total: 2,
        retry_requeued_total: 3,
        ownership_conflicts_total: 5,
        dead_letter_total: 7,
        scm_webhook_received_total: 11,
        scm_webhook_accepted_total: 13,
        scm_webhook_rejected_total: 17,
        scm_webhook_duplicate_total: 19,
        scm_trigger_enqueued_builds_total: 23,
        scm_polling_ticks_total: 29,
        scm_polling_repositories_total: 31,
        scm_polling_enqueued_builds_total: 37,
    };
    storage_a
        .save_runtime_metrics(metrics.clone())
        .await
        .expect("save runtime metrics should succeed");

    storage_a
        .append_scm_webhook_rejection(
            ScmWebhookRejectionRecord {
                reason_code: "forbidden".to_string(),
                provider: Some("github".to_string()),
                repository_url: Some("https://example.com/repo.git".to_string()),
                at: now,
            },
            2,
        )
        .await
        .expect("append webhook rejection should succeed");
    storage_a
        .append_scm_webhook_rejection(
            ScmWebhookRejectionRecord {
                reason_code: "invalid_signature".to_string(),
                provider: Some("gitlab".to_string()),
                repository_url: Some("https://example.com/repo2.git".to_string()),
                at: now + ChronoDuration::seconds(1),
            },
            2,
        )
        .await
        .expect("append webhook rejection should succeed");
    storage_a
        .append_scm_webhook_rejection(
            ScmWebhookRejectionRecord {
                reason_code: "duplicate".to_string(),
                provider: Some("github".to_string()),
                repository_url: Some("https://example.com/repo.git".to_string()),
                at: now + ChronoDuration::seconds(2),
            },
            2,
        )
        .await
        .expect("append webhook rejection should prune oldest rows");

    let storage_b = PostgresStorage::connect(&database_url)
        .await
        .expect("reconnect postgres storage");
    let loaded_metrics = storage_b
        .load_runtime_metrics()
        .await
        .expect("load runtime metrics should succeed");
    assert_eq!(loaded_metrics, metrics);

    let rows = storage_b
        .list_scm_webhook_rejections(10)
        .await
        .expect("list webhook rejections should succeed");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].reason_code, "duplicate");
    assert_eq!(rows[1].reason_code, "invalid_signature");
}
