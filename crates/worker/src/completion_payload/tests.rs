use super::completion_body;
use tardigrade_api::WorkerBuildStatus;

/// Confirms completion payload defaults to success status with log line.
#[test]
fn completion_payload_defaults_to_success_with_log_line() {
    let payload = completion_body();
    assert!(matches!(payload.status, WorkerBuildStatus::Success));
    assert_eq!(
        payload.log_line.as_deref(),
        Some("Completed by tardigrade-worker")
    );
}
