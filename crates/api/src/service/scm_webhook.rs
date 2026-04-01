use axum::http::HeaderMap;
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde_json::Value as JsonValue;
use sha2::Sha256;
use std::time::Duration;
use tardigrade_core::ScmProvider;

use super::ScmTriggerEvent;
use crate::ApiError;

/// Reads one required header value and trims surrounding spaces.
pub(crate) fn header_value(headers: &HeaderMap, key: &'static str) -> Result<String, ApiError> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .ok_or(ApiError::BadRequest)
}

/// Parses provider from unified SCM header.
pub(crate) fn parse_scm_provider_header(headers: &HeaderMap) -> Result<ScmProvider, ApiError> {
    let raw = header_value(headers, "x-scm-provider")?;
    match raw.to_ascii_lowercase().as_str() {
        "github" => Ok(ScmProvider::Github),
        "gitlab" => Ok(ScmProvider::Gitlab),
        _ => Err(ApiError::BadRequest),
    }
}

/// Enforces webhook replay protection using `x-scm-timestamp` unix seconds header.
pub(crate) fn validate_replay_window(headers: &HeaderMap, window: Duration) -> Result<(), ApiError> {
    let raw = header_value(headers, "x-scm-timestamp")?;
    let timestamp = raw.parse::<i64>().map_err(|_| ApiError::Unauthorized)?;
    let now = Utc::now().timestamp();
    let drift = (now - timestamp).unsigned_abs();
    if drift > window.as_secs() {
        return Err(ApiError::Unauthorized);
    }
    Ok(())
}

/// Validates source IP against configured allowlist when list is non-empty.
pub(crate) fn validate_ip_allowlist(
    headers: &HeaderMap,
    allowed_ips: &[String],
) -> Result<(), ApiError> {
    if allowed_ips.is_empty() {
        return Ok(());
    }

    let source_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToString::to_string)
        })
        .ok_or(ApiError::Forbidden)?;

    if allowed_ips.iter().any(|ip| ip == &source_ip) {
        return Ok(());
    }

    Err(ApiError::Forbidden)
}

/// Verifies SCM provider signature semantics for one webhook payload.
pub(crate) fn verify_signature(
    provider: ScmProvider,
    headers: &HeaderMap,
    body: &[u8],
    secret: &str,
) -> Result<(), ApiError> {
    match provider {
        ScmProvider::Github => verify_github_signature(headers, body, secret),
        ScmProvider::Gitlab => verify_gitlab_signature(headers, secret),
    }
}

/// Verifies GitHub `x-hub-signature-256` value against HMAC-SHA256 over request body.
pub(crate) fn verify_github_signature(
    headers: &HeaderMap,
    body: &[u8],
    secret: &str,
) -> Result<(), ApiError> {
    let header = header_value(headers, "x-hub-signature-256").map_err(|_| ApiError::Unauthorized)?;
    let Some(hex_sig) = header.strip_prefix("sha256=") else {
        return Err(ApiError::Unauthorized);
    };

    let provided = hex::decode(hex_sig).map_err(|_| ApiError::Unauthorized)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| ApiError::Internal)?;
    mac.update(body);
    mac.verify_slice(&provided)
        .map_err(|_| ApiError::Unauthorized)
}

/// Verifies GitLab token-style signature header using constant-time equality.
pub(crate) fn verify_gitlab_signature(headers: &HeaderMap, secret: &str) -> Result<(), ApiError> {
    let provided = header_value(headers, "x-gitlab-token").map_err(|_| ApiError::Unauthorized)?;
    if provided.len() != secret.len() {
        return Err(ApiError::Unauthorized);
    }

    let diff = provided
        .as_bytes()
        .iter()
        .zip(secret.as_bytes().iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b));

    if diff == 0 {
        Ok(())
    } else {
        Err(ApiError::Unauthorized)
    }
}

/// Parses provider event metadata into one internal SCM trigger event.
pub(crate) fn parse_scm_trigger_event(
    provider: ScmProvider,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Option<ScmTriggerEvent>, ApiError> {
    match provider {
        ScmProvider::Github => parse_github_trigger_event(headers, body),
        ScmProvider::Gitlab => parse_gitlab_trigger_event(headers, body),
    }
}

/// Builds deterministic dedup key from provider event id or fallback tuple.
pub(crate) fn build_webhook_dedup_key(
    provider: ScmProvider,
    repository_url: &str,
    headers: &HeaderMap,
    body: &[u8],
    event: ScmTriggerEvent,
) -> Option<String> {
    if let Some(event_id) = parse_provider_event_id(provider, headers) {
        return Some(format!(
            "event_id:{}:{}:{}",
            provider_slug(provider),
            repository_url,
            event_id
        ));
    }

    let commit_sha = parse_event_commit_sha(provider, body)
        .unwrap_or_else(|| "unknown_commit".to_string());

    Some(format!(
        "fallback:{}:{}:{}:{}",
        provider_slug(provider),
        repository_url,
        event_slug(event),
        commit_sha
    ))
}

/// Extracts provider event identifier from headers when available.
pub(crate) fn parse_provider_event_id(provider: ScmProvider, headers: &HeaderMap) -> Option<String> {
    let keys: &[&str] = match provider {
        ScmProvider::Github => &["x-scm-event-id", "x-github-delivery", "x-request-id"],
        ScmProvider::Gitlab => &["x-scm-event-id", "x-gitlab-event-uuid", "x-request-id"],
    };

    keys.iter().find_map(|key| optional_header_value(headers, key))
}

/// Parses commit SHA candidates from provider payload for fallback dedup tuple.
pub(crate) fn parse_event_commit_sha(provider: ScmProvider, body: &[u8]) -> Option<String> {
    let payload: JsonValue = serde_json::from_slice(body).ok()?;
    match provider {
        ScmProvider::Github => payload
            .get("after")
            .and_then(JsonValue::as_str)
            .or_else(|| {
                payload
                    .get("head_commit")
                    .and_then(|v| v.get("id"))
                    .and_then(JsonValue::as_str)
            })
            .or_else(|| {
                payload
                    .get("pull_request")
                    .and_then(|v| v.get("head"))
                    .and_then(|v| v.get("sha"))
                    .and_then(JsonValue::as_str)
            })
            .map(ToString::to_string),
        ScmProvider::Gitlab => payload
            .get("checkout_sha")
            .and_then(JsonValue::as_str)
            .or_else(|| {
                payload
                    .get("object_attributes")
                    .and_then(|v| v.get("last_commit"))
                    .and_then(|v| v.get("id"))
                    .and_then(JsonValue::as_str)
            })
            .map(ToString::to_string),
    }
}

/// Returns lowercased header value when present and non-empty.
pub(crate) fn optional_header_value(headers: &HeaderMap, key: &'static str) -> Option<String> {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_ascii_lowercase())
}

/// Returns stable provider slug used in dedup key encoding.
pub(crate) fn provider_slug(provider: ScmProvider) -> &'static str {
    match provider {
        ScmProvider::Github => "github",
        ScmProvider::Gitlab => "gitlab",
    }
}

/// Returns stable trigger family slug used in fallback dedup key encoding.
pub(crate) fn event_slug(event: ScmTriggerEvent) -> &'static str {
    match event {
        ScmTriggerEvent::Push => "push",
        ScmTriggerEvent::PullRequest => "pull_request",
        ScmTriggerEvent::MergeRequest => "merge_request",
        ScmTriggerEvent::Tag => "tag",
        ScmTriggerEvent::ManualDispatch => "manual_dispatch",
    }
}

/// Maps GitHub webhook event headers/payload into internal trigger family.
pub(crate) fn parse_github_trigger_event(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Option<ScmTriggerEvent>, ApiError> {
    let event_name = header_value(headers, "x-github-event")?.to_ascii_lowercase();
    if event_name == "push" {
        let payload: JsonValue =
            serde_json::from_slice(body).map_err(|_| ApiError::BadRequest)?;
        let is_tag = payload
            .get("ref")
            .and_then(JsonValue::as_str)
            .map(|r| r.starts_with("refs/tags/"))
            .unwrap_or(false);
        return Ok(Some(if is_tag {
            ScmTriggerEvent::Tag
        } else {
            ScmTriggerEvent::Push
        }));
    }

    if event_name == "pull_request" {
        return Ok(Some(ScmTriggerEvent::PullRequest));
    }

    if event_name == "workflow_dispatch" {
        return Ok(Some(ScmTriggerEvent::ManualDispatch));
    }

    Ok(None)
}

/// Maps GitLab webhook event headers/payload into internal trigger family.
pub(crate) fn parse_gitlab_trigger_event(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Option<ScmTriggerEvent>, ApiError> {
    let event_name = header_value(headers, "x-gitlab-event")?.to_ascii_lowercase();
    if event_name == "push hook" {
        return Ok(Some(ScmTriggerEvent::Push));
    }

    if event_name == "merge request hook" {
        return Ok(Some(ScmTriggerEvent::MergeRequest));
    }

    if event_name == "tag push hook" {
        return Ok(Some(ScmTriggerEvent::Tag));
    }

    if event_name == "pipeline hook" {
        let payload: JsonValue =
            serde_json::from_slice(body).map_err(|_| ApiError::BadRequest)?;
        let source = payload
            .get("object_attributes")
            .and_then(|v| v.get("source"))
            .and_then(JsonValue::as_str)
            .map(|v| v.to_ascii_lowercase());
        if source.as_deref() == Some("web") {
            return Ok(Some(ScmTriggerEvent::ManualDispatch));
        }
        return Ok(None);
    }

    Ok(None)
}
