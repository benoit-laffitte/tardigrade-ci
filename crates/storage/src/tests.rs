use super::{InMemoryStorage, Storage, parse_status, status_to_str};
use tardigrade_core::{BuildRecord, JobDefinition, JobStatus};

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

#[test]
fn status_helpers_cover_all_supported_values() {
    let statuses = [
        JobStatus::Pending,
        JobStatus::Running,
        JobStatus::Success,
        JobStatus::Failed,
        JobStatus::Canceled,
    ];

    for status in statuses {
        let raw = status_to_str(&status);
        let parsed = parse_status(raw).expect("parse should succeed");
        assert_eq!(parsed, status);
    }
}

#[test]
fn parse_status_rejects_unknown_values() {
    let err = parse_status("unknown").expect_err("unknown status should fail");
    assert!(err.to_string().contains("unknown job status"));
}
