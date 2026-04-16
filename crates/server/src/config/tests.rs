use std::time::{SystemTime, UNIX_EPOCH};

use super::{RuntimeMode, load_runtime_mode_from_config, parse_runtime_mode_from_toml};

/// Parses explicit prod mode from TOML runtime section.
#[test]
fn parse_runtime_mode_reads_prod_value() {
    let raw = r#"
[runtime]
mode = "prod"
"#;

    let mode = parse_runtime_mode_from_toml(raw).expect("parse runtime mode");
    assert_eq!(mode, RuntimeMode::Prod);
}

/// Defaults to dev mode when runtime section is omitted.
#[test]
fn parse_runtime_mode_defaults_to_dev_when_runtime_missing() {
    let raw = r#"
[server]
bind = "127.0.0.1:8080"
"#;

    let mode = parse_runtime_mode_from_toml(raw).expect("parse runtime mode");
    assert_eq!(mode, RuntimeMode::Dev);
}

/// Missing config file path returns an explicit read error.
#[test]
fn load_runtime_mode_fails_when_file_is_missing() {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("unix epoch")
        .as_nanos();
    let missing_path = format!("/tmp/tardigrade-missing-config-{unique_suffix}.toml");
    let error = load_runtime_mode_from_config(&missing_path)
        .expect_err("missing config should return an error");
    assert!(error.to_string().contains("read config file"));
}
