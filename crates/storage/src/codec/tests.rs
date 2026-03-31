use super::{parse_scm_provider, parse_status, scm_provider_to_str, status_to_str};
use tardigrade_core::{JobStatus, ScmProvider};

/// Verifies status codec roundtrip for all supported status values.
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

/// Verifies unknown status values are rejected.
#[test]
fn parse_status_rejects_unknown_values() {
    let err = parse_status("unknown").expect_err("unknown status should fail");
    assert!(err.to_string().contains("unknown job status"));
}

/// Verifies scm provider codec roundtrip for supported providers.
#[test]
fn scm_provider_helpers_cover_all_supported_values() {
    let providers = [ScmProvider::Github, ScmProvider::Gitlab];

    for provider in providers {
        let raw = scm_provider_to_str(provider);
        let parsed = parse_scm_provider(raw).expect("parse should succeed");
        assert_eq!(parsed, provider);
    }
}

/// Verifies unknown SCM provider values are rejected.
#[test]
fn parse_scm_provider_rejects_unknown_values() {
    let err = parse_scm_provider("bitbucket").expect_err("unknown provider should fail");
    assert!(err.to_string().contains("unknown SCM provider"));
}
