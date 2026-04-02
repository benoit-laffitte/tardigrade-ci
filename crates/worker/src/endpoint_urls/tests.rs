use super::{claim_url, complete_url};
use uuid::Uuid;

/// Confirms worker endpoint URLs are rendered consistently.
#[test]
fn worker_urls_are_built_consistently() {
    let server_url = "http://127.0.0.1:8080";
    let worker_id = "worker-a";
    let build_id = Uuid::parse_str("00000000-0000-0000-0000-000000000123").expect("valid uuid");

    assert_eq!(
        claim_url(server_url, worker_id),
        "http://127.0.0.1:8080/workers/worker-a/claim"
    );
    assert_eq!(
        complete_url(server_url, worker_id, build_id),
        "http://127.0.0.1:8080/workers/worker-a/builds/00000000-0000-0000-0000-000000000123/complete"
    );
}
