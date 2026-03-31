use serde::Deserialize;

/// Runtime mode derived from configuration file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeMode {
    Dev,
    Prod,
}

impl Default for RuntimeMode {
    /// Defaults runtime mode to dev when configuration omits explicit value.
    fn default() -> Self {
        Self::Dev
    }
}
