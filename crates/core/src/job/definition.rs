use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Immutable job declaration describing where code lives and which pipeline to run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDefinition {
    pub id: Uuid,
    pub name: String,
    pub repository_url: String,
    pub pipeline_path: String,
    pub created_at: DateTime<Utc>,
}

impl JobDefinition {
    /// Creates a new job definition with generated id and creation timestamp.
    pub fn new(
        name: impl Into<String>,
        repository_url: impl Into<String>,
        pipeline_path: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            repository_url: repository_url.into(),
            pipeline_path: pipeline_path.into(),
            created_at: Utc::now(),
        }
    }
}
