use serde::Deserialize;

use super::RuntimeMode;

/// Runtime-specific configuration section.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct RuntimeSection {
    pub mode: RuntimeMode,
}

impl Default for RuntimeSection {
    /// Defaults runtime settings to dev mode when omitted in TOML.
    fn default() -> Self {
        Self {
            mode: RuntimeMode::Dev,
        }
    }
}
