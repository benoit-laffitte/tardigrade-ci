use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{WorkerConfig, load_worker_config};

/// Confirms worker configuration defaults remain stable.
#[test]
fn worker_config_defaults_are_stable() {
    let defaults = WorkerConfig::default();
    assert_eq!(defaults.server_url, "http://127.0.0.1:8080");
    assert_eq!(defaults.worker_id, "worker-local");
    assert_eq!(defaults.poll_ms, 250);
    assert!(defaults.http2_enabled);
    assert!(!defaults.http2_prior_knowledge);
    assert_eq!(defaults.request_timeout_secs, 30);
    assert_eq!(defaults.pool_idle_timeout_secs, 90);
    assert_eq!(defaults.pool_max_idle_per_host, 32);
    assert_eq!(defaults.http2_keep_alive_secs, 30);
}

/// Confirms explicit TOML worker configuration values are preserved.
#[test]
fn worker_config_uses_provided_values() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("unix epoch")
        .as_nanos();
    let path = format!("/tmp/tardigrade-worker-config-{suffix}.toml");
    let raw = r#"
[worker]
server_url = "http://ci.internal:8080"
worker_id = "worker-a"
poll_ms = 500
http2_enabled = false
http2_prior_knowledge = true
request_timeout_secs = 15
pool_idle_timeout_secs = 20
pool_max_idle_per_host = 8
http2_keep_alive_secs = 7
"#;
    fs::write(&path, raw).expect("write temp worker config");

    let cfg = load_worker_config(&path).expect("load worker config");

    assert_eq!(cfg.server_url, "http://ci.internal:8080");
    assert_eq!(cfg.worker_id, "worker-a");
    assert_eq!(cfg.poll_ms, 500);
    assert!(!cfg.http2_enabled);
    assert!(cfg.http2_prior_knowledge);
    assert_eq!(cfg.request_timeout_secs, 15);
    assert_eq!(cfg.pool_idle_timeout_secs, 20);
    assert_eq!(cfg.pool_max_idle_per_host, 8);
    assert_eq!(cfg.http2_keep_alive_secs, 7);

    let _ = fs::remove_file(path);
}

/// Confirms zero values are normalized to safe defaults.
#[test]
fn worker_config_normalizes_zero_values() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("unix epoch")
        .as_nanos();
    let path = format!("/tmp/tardigrade-worker-config-zero-{suffix}.toml");
    let raw = r#"
[worker]
poll_ms = 0
request_timeout_secs = 0
pool_idle_timeout_secs = 0
pool_max_idle_per_host = 0
http2_keep_alive_secs = 0
"#;
    fs::write(&path, raw).expect("write temp worker config");

    let cfg = load_worker_config(&path).expect("load worker config");

    assert_eq!(cfg.poll_ms, 250);
    assert_eq!(cfg.request_timeout_secs, 30);
    assert_eq!(cfg.pool_idle_timeout_secs, 90);
    assert_eq!(cfg.pool_max_idle_per_host, 32);
    assert_eq!(cfg.http2_keep_alive_secs, 30);

    let _ = fs::remove_file(path);
}

/// Confirms missing worker section falls back to default values.
#[test]
fn load_worker_config_uses_defaults_when_section_missing() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("unix epoch")
        .as_nanos();
    let path = format!("/tmp/tardigrade-worker-config-default-{suffix}.toml");
    let raw = r#"
[runtime]
mode = "dev"
"#;
    fs::write(&path, raw).expect("write temp worker config");

    let cfg = load_worker_config(&path).expect("load worker config");

    assert!(!cfg.server_url.trim().is_empty());
    assert!(!cfg.worker_id.trim().is_empty());
    assert!(cfg.poll_ms > 0);
    assert!(cfg.request_timeout_secs > 0);
    assert!(cfg.pool_idle_timeout_secs > 0);
    assert!(cfg.pool_max_idle_per_host > 0);
    assert!(cfg.http2_keep_alive_secs > 0);

    let _ = fs::remove_file(path);
}
