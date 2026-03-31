use serde::{Deserialize, Serialize};

/// SCM provider identity used for webhook signature verification behavior.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ScmProvider {
    Github,
    Gitlab,
}
