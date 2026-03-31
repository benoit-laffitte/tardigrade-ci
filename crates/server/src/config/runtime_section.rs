use serde::Deserialize;

use super::RuntimeMode;

/// Runtime-specific configuration section.
#[derive(Debug, Deserialize)]
pub struct RuntimeSection {
    pub mode: RuntimeMode,
}
