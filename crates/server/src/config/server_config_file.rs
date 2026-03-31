use serde::Deserialize;

use super::RuntimeSection;

/// Top-level config file shape used by server bootstrap.
#[derive(Debug, Deserialize, Default)]
pub struct ServerConfigFile {
    pub runtime: Option<RuntimeSection>,
}
