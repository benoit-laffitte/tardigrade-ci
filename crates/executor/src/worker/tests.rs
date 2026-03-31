use tardigrade_core::{BuildRecord, JobDefinition, JobStatus};

use super::WorkerExecutor;

/// Verifies worker executor marks running build as success and appends logs.
#[tokio::test]
async fn worker_executor_run_marks_success_and_appends_logs() {
    let job = JobDefinition::new(
        "executor-test",
        "https://example.com/repo.git",
        "pipeline.yml",
    );
    let mut build = BuildRecord::queued(job.id);
    assert!(build.mark_running());

    let updated = WorkerExecutor::run(build)
        .await
        .expect("executor run should succeed");

    assert!(updated.logs.iter().any(|line| line.contains("started")));
    assert!(updated.logs.iter().any(|line| line.contains("finished")));
    assert_eq!(updated.status, JobStatus::Success);
}
