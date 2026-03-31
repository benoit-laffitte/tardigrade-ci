use serde::{Deserialize, Serialize};

/// Supported high-level language families for technology execution profiles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TechnologyLanguage {
    Rust,
    Python,
    Java,
    Node,
    Go,
}
