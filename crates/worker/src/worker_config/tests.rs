use super::{
    load_worker_config, parse_bool, parse_poll_ms, parse_u64, parse_usize, resolve_server_url,
    resolve_worker_id,
};

/// Confirms worker configuration defaults remain stable.
#[test]
fn worker_config_defaults_are_stable() {
    assert_eq!(resolve_server_url(None), "http://127.0.0.1:8080");
    assert_eq!(resolve_worker_id(None), "worker-local");
    assert_eq!(parse_poll_ms(None), 250);
    assert!(parse_bool(None, true));
    assert_eq!(parse_u64(None, 42), 42);
    assert_eq!(parse_usize(None, 7), 7);
}

/// Confirms explicit worker configuration values are preserved.
#[test]
fn worker_config_uses_provided_values() {
    assert_eq!(
        resolve_server_url(Some("http://ci.internal:8080")),
        "http://ci.internal:8080"
    );
    assert_eq!(resolve_worker_id(Some("worker-a")), "worker-a");
    assert_eq!(parse_poll_ms(Some("500")), 500);
}

/// Confirms invalid poll interval falls back to default value.
#[test]
fn worker_config_rejects_invalid_poll_value() {
    assert_eq!(parse_poll_ms(Some("not-a-number")), 250);
}

/// Confirms bool parser accepts common true/false aliases.
#[test]
fn worker_config_bool_parser_supports_aliases() {
    assert!(parse_bool(Some("true"), false));
    assert!(parse_bool(Some("1"), false));
    assert!(!parse_bool(Some("false"), true));
    assert!(!parse_bool(Some("off"), true));
}

/// Confirms numeric parser falls back to defaults on invalid values.
#[test]
fn worker_config_numeric_parser_rejects_invalid_values() {
    assert_eq!(parse_u64(Some("invalid"), 30), 30);
    assert_eq!(parse_usize(Some("invalid"), 32), 32);
}

/// Confirms environment-based config loading always produces usable values.
#[test]
fn load_worker_config_produces_valid_values() {
    let cfg = load_worker_config();
    assert!(!cfg.server_url.trim().is_empty());
    assert!(!cfg.worker_id.trim().is_empty());
    assert!(cfg.poll_ms > 0);
    assert!(cfg.request_timeout_secs > 0);
    assert!(cfg.pool_idle_timeout_secs > 0);
    assert!(cfg.pool_max_idle_per_host > 0);
    assert!(cfg.http2_keep_alive_secs > 0);
}
