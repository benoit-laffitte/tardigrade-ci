use serde::{Deserialize, Serialize};

/// Runtime metadata used to select container and shell behavior for one profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeMetadata {
    pub image: String,
    #[serde(default)]
    pub shell: Option<String>,
}
