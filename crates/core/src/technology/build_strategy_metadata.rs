use serde::{Deserialize, Serialize};

/// Build strategy metadata used by orchestration layers to generate executable steps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildStrategyMetadata {
    #[serde(default)]
    pub install: Vec<String>,
    #[serde(default)]
    pub build: Vec<String>,
    #[serde(default)]
    pub test: Vec<String>,
    #[serde(default)]
    pub package: Vec<String>,
}
