use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use tardigrade_api::{
    ApiState, CreateJobResponse, ListBuildsResponse, ScmWebhookAcceptedResponse,
    UpsertWebhookSecurityConfigRequest,
};
use tardigrade_core::ScmProvider;
use tower::ServiceExt;

/// Builds one GitHub-style `sha256=<hex>` signature over raw request body.
fn github_signature(secret: &str, body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("hmac init");
    mac.update(body);
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

/// Reads JSON body from an axum response.
async fn read_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&body).expect("valid json")
}

#[tokio::test]
/// Accepts a valid GitHub webhook when signature, replay window, and allowlist all match.
async fn scm_webhook_github_valid_signature_is_accepted() {
    let state = ApiState::new("tardigrade-ci-test");
    state
        .upsert_webhook_security_config(UpsertWebhookSecurityConfigRequest {
            repository_url: "https://example.com/repo.git".to_string(),
            provider: ScmProvider::Github,
            secret: "super-secret".to_string(),
            allowed_ips: vec!["203.0.113.10".to_string()],
        })
        .await
        .expect("upsert webhook config");
    let app = tardigrade_api::build_router(state);

    let payload = br#"{"event":"push"}"#;
    let signature = github_signature("super-secret", payload);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks/scm")
                .header("x-scm-provider", "github")
                .header("x-scm-repository", "https://example.com/repo.git")
                .header("x-scm-timestamp", Utc::now().timestamp().to_string())
                .header("x-forwarded-for", "203.0.113.10")
                .header("x-github-event", "push")
                .header("x-hub-signature-256", signature)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_vec()))
                .expect("valid request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let accepted: ScmWebhookAcceptedResponse = serde_json::from_value(read_json(response).await)
        .expect("accepted body");
    assert_eq!(accepted.status, "accepted");
}

#[tokio::test]
/// Rejects missing signature in strict mode.
async fn scm_webhook_missing_signature_is_unauthorized() {
    let state = ApiState::new("tardigrade-ci-test");
    state
        .upsert_webhook_security_config(UpsertWebhookSecurityConfigRequest {
            repository_url: "https://example.com/repo.git".to_string(),
            provider: ScmProvider::Github,
            secret: "super-secret".to_string(),
            allowed_ips: vec!["203.0.113.10".to_string()],
        })
        .await
        .expect("upsert webhook config");
    let app = tardigrade_api::build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks/scm")
                .header("x-scm-provider", "github")
                .header("x-scm-repository", "https://example.com/repo.git")
                .header("x-scm-timestamp", Utc::now().timestamp().to_string())
                .header("x-forwarded-for", "203.0.113.10")
                .body(Body::from(br#"{"event":"push"}"#.to_vec()))
                .expect("valid request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
/// Rejects webhooks outside the configured anti-replay window.
async fn scm_webhook_expired_timestamp_is_unauthorized() {
    let state = ApiState::new("tardigrade-ci-test");
    state
        .upsert_webhook_security_config(UpsertWebhookSecurityConfigRequest {
            repository_url: "https://example.com/repo.git".to_string(),
            provider: ScmProvider::Github,
            secret: "super-secret".to_string(),
            allowed_ips: vec!["203.0.113.10".to_string()],
        })
        .await
        .expect("upsert webhook config");
    let app = tardigrade_api::build_router(state);

    let payload = br#"{"event":"push"}"#;
    let signature = github_signature("super-secret", payload);
    let stale_ts = Utc::now().timestamp() - 3600;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks/scm")
                .header("x-scm-provider", "github")
                .header("x-scm-repository", "https://example.com/repo.git")
                .header("x-scm-timestamp", stale_ts.to_string())
                .header("x-forwarded-for", "203.0.113.10")
                .header("x-hub-signature-256", signature)
                .body(Body::from(payload.to_vec()))
                .expect("valid request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
/// Rejects webhooks from an IP outside repository allowlist.
async fn scm_webhook_ip_outside_allowlist_is_forbidden() {
    let state = ApiState::new("tardigrade-ci-test");
    state
        .upsert_webhook_security_config(UpsertWebhookSecurityConfigRequest {
            repository_url: "https://example.com/repo.git".to_string(),
            provider: ScmProvider::Github,
            secret: "super-secret".to_string(),
            allowed_ips: vec!["203.0.113.10".to_string()],
        })
        .await
        .expect("upsert webhook config");
    let app = tardigrade_api::build_router(state);

    let payload = br#"{"event":"push"}"#;
    let signature = github_signature("super-secret", payload);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks/scm")
                .header("x-scm-provider", "github")
                .header("x-scm-repository", "https://example.com/repo.git")
                .header("x-scm-timestamp", Utc::now().timestamp().to_string())
                .header("x-forwarded-for", "198.51.100.20")
                .header("x-hub-signature-256", signature)
                .body(Body::from(payload.to_vec()))
                .expect("valid request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
/// Accepts GitLab webhook when token matches configured repository secret.
async fn scm_webhook_gitlab_token_is_accepted() {
    let state = ApiState::new("tardigrade-ci-test");
    state
        .upsert_webhook_security_config(UpsertWebhookSecurityConfigRequest {
            repository_url: "https://gitlab.example.com/group/repo.git".to_string(),
            provider: ScmProvider::Gitlab,
            secret: "gitlab-secret".to_string(),
            allowed_ips: vec!["192.0.2.42".to_string()],
        })
        .await
        .expect("upsert webhook config");
    let app = tardigrade_api::build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks/scm")
                .header("x-scm-provider", "gitlab")
                .header(
                    "x-scm-repository",
                    "https://gitlab.example.com/group/repo.git",
                )
                .header("x-scm-timestamp", Utc::now().timestamp().to_string())
                .header("x-forwarded-for", "192.0.2.42")
                .header("x-gitlab-event", "Push Hook")
                .header("x-gitlab-token", "gitlab-secret")
                .body(Body::from(br#"{"event":"push"}"#.to_vec()))
                .expect("valid request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
/// Ensures GitHub push adapter enqueues a build for matching repository jobs.
async fn scm_webhook_github_push_enqueues_build() {
    let state = ApiState::new("tardigrade-ci-test");
    state
        .upsert_webhook_security_config(UpsertWebhookSecurityConfigRequest {
            repository_url: "https://example.com/repo.git".to_string(),
            provider: ScmProvider::Github,
            secret: "super-secret".to_string(),
            allowed_ips: vec!["203.0.113.10".to_string()],
        })
        .await
        .expect("upsert webhook config");
    let app = tardigrade_api::build_router(state);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    br#"{"name":"repo-build","repository_url":"https://example.com/repo.git","pipeline_path":"pipeline.yml"}"#.to_vec(),
                ))
                .expect("valid request"),
        )
        .await
        .expect("create response");
    let created: CreateJobResponse =
        serde_json::from_value(read_json(create_response).await).expect("create body");

    let payload = br#"{"ref":"refs/heads/main"}"#;
    let signature = github_signature("super-secret", payload);

    let webhook_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks/scm")
                .header("x-scm-provider", "github")
                .header("x-scm-repository", "https://example.com/repo.git")
                .header("x-scm-timestamp", Utc::now().timestamp().to_string())
                .header("x-forwarded-for", "203.0.113.10")
                .header("x-github-event", "push")
                .header("x-hub-signature-256", signature)
                .body(Body::from(payload.to_vec()))
                .expect("valid request"),
        )
        .await
        .expect("webhook response");

    assert_eq!(webhook_response.status(), StatusCode::ACCEPTED);

    let builds_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/builds")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("builds response");
    let builds: ListBuildsResponse =
        serde_json::from_value(read_json(builds_response).await).expect("builds body");

    assert!(
        builds
            .builds
            .iter()
            .any(|build| build.job_id == created.job.id),
        "expected one build enqueued for matching repository"
    );
}

#[tokio::test]
/// Ensures GitLab merge request adapter enqueues a build for matching repository jobs.
async fn scm_webhook_gitlab_merge_request_enqueues_build() {
    let state = ApiState::new("tardigrade-ci-test");
    state
        .upsert_webhook_security_config(UpsertWebhookSecurityConfigRequest {
            repository_url: "https://gitlab.example.com/group/repo.git".to_string(),
            provider: ScmProvider::Gitlab,
            secret: "gitlab-secret".to_string(),
            allowed_ips: vec!["192.0.2.42".to_string()],
        })
        .await
        .expect("upsert webhook config");
    let app = tardigrade_api::build_router(state);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    br#"{"name":"repo-build","repository_url":"https://gitlab.example.com/group/repo.git","pipeline_path":"pipeline.yml"}"#.to_vec(),
                ))
                .expect("valid request"),
        )
        .await
        .expect("create response");
    let created: CreateJobResponse =
        serde_json::from_value(read_json(create_response).await).expect("create body");

    let webhook_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks/scm")
                .header("x-scm-provider", "gitlab")
                .header(
                    "x-scm-repository",
                    "https://gitlab.example.com/group/repo.git",
                )
                .header("x-scm-timestamp", Utc::now().timestamp().to_string())
                .header("x-forwarded-for", "192.0.2.42")
                .header("x-gitlab-event", "Merge Request Hook")
                .header("x-gitlab-token", "gitlab-secret")
                .body(Body::from(br#"{"object_kind":"merge_request"}"#.to_vec()))
                .expect("valid request"),
        )
        .await
        .expect("webhook response");

    assert_eq!(webhook_response.status(), StatusCode::ACCEPTED);

    let builds_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/builds")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("builds response");
    let builds: ListBuildsResponse =
        serde_json::from_value(read_json(builds_response).await).expect("builds body");

    assert!(
        builds
            .builds
            .iter()
            .any(|build| build.job_id == created.job.id),
        "expected one build enqueued for matching repository"
    );
}
