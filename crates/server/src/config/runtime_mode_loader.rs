use anyhow::{Context, Result};
use std::fs;

use super::{RuntimeMode, ServerConfigFile};

/// Parses runtime mode from TOML payload.
pub fn parse_runtime_mode_from_toml(raw: &str) -> Result<RuntimeMode> {
    let config: ServerConfigFile = toml::from_str(raw).context("parse TOML configuration")?;
    Ok(config.runtime.mode)
}

/// Loads runtime mode from config file path.
pub fn load_runtime_mode_from_config(path: &str) -> Result<RuntimeMode> {
    let raw = fs::read_to_string(path).with_context(|| format!("read config file at {path}"))?;
    parse_runtime_mode_from_toml(&raw)
}
