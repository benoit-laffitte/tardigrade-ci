use anyhow::{Context, Result};
use std::fs;
use tracing::info;

use super::{RuntimeMode, ServerConfigFile};

/// Parses runtime mode from TOML payload.
pub fn parse_runtime_mode_from_toml(raw: &str) -> Result<RuntimeMode> {
    let config: ServerConfigFile = toml::from_str(raw).context("parse TOML configuration")?;
    Ok(config
        .runtime
        .map(|runtime| runtime.mode)
        .unwrap_or_default())
}

/// Loads runtime mode from config file path, defaulting to dev when file is missing.
pub fn load_runtime_mode_from_config(path: &str) -> Result<RuntimeMode> {
    match fs::read_to_string(path) {
        Ok(raw) => parse_runtime_mode_from_toml(&raw),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            info!(config_path = %path, "config file not found, defaulting runtime mode to dev");
            Ok(RuntimeMode::Dev)
        }
        Err(err) => Err(err).with_context(|| format!("read config file at {path}")),
    }
}
