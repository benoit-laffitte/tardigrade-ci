use serde::Deserialize;

/// Request payload used to create a new job.
#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub name: String,
    pub repository_url: String,
    pub pipeline_path: String,
    pub pipeline_yaml: Option<String>,
}
