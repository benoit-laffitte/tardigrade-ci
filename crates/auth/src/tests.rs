use super::ApiKeyAuth;

/// Confirms key verifier accepts expected key and rejects mismatched one.
#[test]
fn verifies_correct_key() {
    let auth = ApiKeyAuth::new("secret");
    assert!(auth.verify("secret"));
    assert!(!auth.verify("wrong"));
}
