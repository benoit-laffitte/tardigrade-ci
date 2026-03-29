use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use uuid::Uuid;

/// Lifecycle status for a build execution in the CI control-plane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobStatus {
    /// Pending means queued but not yet claimed by a worker.
    Pending,
    /// Running means a worker currently owns and executes the build.
    Running,
    /// Success/Failed/Canceled are terminal states.
    Success,
    Failed,
    Canceled,
}

/// Immutable job declaration describing where code lives and which pipeline to run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDefinition {
    pub id: Uuid,
    pub name: String,
    pub repository_url: String,
    pub pipeline_path: String,
    pub created_at: DateTime<Utc>,
}

/// Versioned pipeline schema executed by workers for one job definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineDefinition {
    pub version: u32,
    pub stages: Vec<PipelineStage>,
}

/// Ordered stage grouping one or more executable steps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineStage {
    pub name: String,
    pub steps: Vec<PipelineStep>,
}

/// Single execution unit in a stage with optional retry override.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineStep {
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub retry: Option<PipelineRetryPolicy>,
}

/// Retry policy hook allowing DSL-level override per step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineRetryPolicy {
    pub max_attempts: u32,
    pub backoff_ms: u64,
}

/// One actionable validation issue found in a pipeline definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineValidationIssue {
    pub field: String,
    pub message: String,
}

/// Error returned when parsing or validating pipeline DSL input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineDslError {
    Yaml(String),
    Validation(Vec<PipelineValidationIssue>),
}

impl fmt::Display for PipelineDslError {
    /// Renders parser/validator failures in operator-friendly text form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yaml(message) => write!(f, "invalid YAML pipeline definition: {message}"),
            Self::Validation(issues) => write!(
                f,
                "invalid pipeline definition ({} structural issue(s))",
                issues.len()
            ),
        }
    }
}

impl std::error::Error for PipelineDslError {}

/// Mutable runtime record tracking one execution attempt of a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRecord {
    pub id: Uuid,
    pub job_id: Uuid,
    pub status: JobStatus,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub logs: Vec<String>,
}

impl JobDefinition {
    /// Creates a new job definition with generated id and creation timestamp.
    pub fn new(name: impl Into<String>, repository_url: impl Into<String>, pipeline_path: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            repository_url: repository_url.into(),
            pipeline_path: pipeline_path.into(),
            created_at: Utc::now(),
        }
    }
}

impl PipelineDefinition {
    /// Creates pipeline definition with schema version 1.
    pub fn v1(stages: Vec<PipelineStage>) -> Self {
        Self { version: 1, stages }
    }

    /// Parses one YAML pipeline definition and validates its structure.
    pub fn from_yaml_str(raw: &str) -> Result<Self, PipelineDslError> {
        let parsed: Self =
            serde_yaml::from_str(raw).map_err(|error| PipelineDslError::Yaml(error.to_string()))?;
        parsed.validate().map_err(PipelineDslError::Validation)?;
        Ok(parsed)
    }

    /// Validates schema invariants required for deterministic execution.
    pub fn validate(&self) -> Result<(), Vec<PipelineValidationIssue>> {
        let mut issues = Vec::new();

        if self.version != 1 {
            issues.push(PipelineValidationIssue {
                field: "version".to_string(),
                message: "only schema version 1 is supported".to_string(),
            });
        }

        if self.stages.is_empty() {
            issues.push(PipelineValidationIssue {
                field: "stages".to_string(),
                message: "at least one stage is required".to_string(),
            });
        }

        let mut stage_names = std::collections::BTreeSet::new();
        for (stage_idx, stage) in self.stages.iter().enumerate() {
            let stage_field = format!("stages[{stage_idx}]");
            let stage_name = stage.name.trim();

            if stage_name.is_empty() {
                issues.push(PipelineValidationIssue {
                    field: format!("{stage_field}.name"),
                    message: "stage name cannot be empty".to_string(),
                });
            } else if !stage_names.insert(stage_name.to_string()) {
                issues.push(PipelineValidationIssue {
                    field: format!("{stage_field}.name"),
                    message: format!("duplicate stage name '{stage_name}'"),
                });
            }

            if stage.steps.is_empty() {
                issues.push(PipelineValidationIssue {
                    field: format!("{stage_field}.steps"),
                    message: "at least one step is required in each stage".to_string(),
                });
            }

            let mut step_names = std::collections::BTreeSet::new();
            for (step_idx, step) in stage.steps.iter().enumerate() {
                let step_field = format!("{stage_field}.steps[{step_idx}]");
                let step_name = step.name.trim();

                if step_name.is_empty() {
                    issues.push(PipelineValidationIssue {
                        field: format!("{step_field}.name"),
                        message: "step name cannot be empty".to_string(),
                    });
                } else if !step_names.insert(step_name.to_string()) {
                    issues.push(PipelineValidationIssue {
                        field: format!("{step_field}.name"),
                        message: format!(
                            "duplicate step name '{step_name}' in stage '{}'",
                            stage.name
                        ),
                    });
                }

                if step.image.trim().is_empty() {
                    issues.push(PipelineValidationIssue {
                        field: format!("{step_field}.image"),
                        message: "step image cannot be empty".to_string(),
                    });
                }

                if step.command.is_empty() {
                    issues.push(PipelineValidationIssue {
                        field: format!("{step_field}.command"),
                        message: "step command must contain at least one token".to_string(),
                    });
                }

                for (command_idx, token) in step.command.iter().enumerate() {
                    if token.trim().is_empty() {
                        issues.push(PipelineValidationIssue {
                            field: format!("{step_field}.command[{command_idx}]"),
                            message: "command tokens cannot be empty".to_string(),
                        });
                    }
                }

                for (key, value) in &step.env {
                    if key.trim().is_empty() {
                        issues.push(PipelineValidationIssue {
                            field: format!("{step_field}.env"),
                            message: "environment variable keys cannot be empty".to_string(),
                        });
                    }

                    if value.trim().is_empty() {
                        issues.push(PipelineValidationIssue {
                            field: format!("{step_field}.env.{key}"),
                            message: "environment variable values cannot be empty".to_string(),
                        });
                    }
                }

                if let Some(retry) = &step.retry {
                    if retry.max_attempts == 0 {
                        issues.push(PipelineValidationIssue {
                            field: format!("{step_field}.retry.max_attempts"),
                            message: "retry max_attempts must be greater than 0".to_string(),
                        });
                    }

                    if retry.backoff_ms == 0 {
                        issues.push(PipelineValidationIssue {
                            field: format!("{step_field}.retry.backoff_ms"),
                            message: "retry backoff_ms must be greater than 0".to_string(),
                        });
                    }
                }
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

impl PipelineStage {
    /// Creates one pipeline stage with ordered steps.
    pub fn new(name: impl Into<String>, steps: Vec<PipelineStep>) -> Self {
        Self {
            name: name.into(),
            steps,
        }
    }
}

impl PipelineStep {
    /// Creates one pipeline step with image and command.
    pub fn new(name: impl Into<String>, image: impl Into<String>, command: Vec<String>) -> Self {
        Self {
            name: name.into(),
            image: image.into(),
            command,
            env: BTreeMap::new(),
            retry: None,
        }
    }
}

impl PipelineRetryPolicy {
    /// Creates retry policy with max attempts and linear backoff in milliseconds.
    pub fn new(max_attempts: u32, backoff_ms: u64) -> Self {
        Self {
            max_attempts,
            backoff_ms,
        }
    }
}

impl BuildRecord {
    /// Creates a newly queued build record tied to a job id.
    pub fn queued(job_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            job_id,
            status: JobStatus::Pending,
            queued_at: Utc::now(),
            started_at: None,
            finished_at: None,
            logs: Vec::new(),
        }
    }

    /// Marks the build as running when claimed by a worker.
    pub fn mark_running(&mut self) -> bool {
        // State transition guard: only pending builds can be claimed.
        if self.status != JobStatus::Pending {
            return false;
        }

        self.status = JobStatus::Running;
        self.started_at = Some(Utc::now());
        true
    }

    /// Marks the build as successful after worker completion.
    pub fn mark_success(&mut self) -> bool {
        // State transition guard: success can only come from running.
        if self.status != JobStatus::Running {
            return false;
        }

        self.status = JobStatus::Success;
        self.finished_at = Some(Utc::now());
        true
    }

    /// Marks the build as failed after worker completion.
    pub fn mark_failed(&mut self) -> bool {
        // State transition guard: failure can only come from running.
        if self.status != JobStatus::Running {
            return false;
        }

        self.status = JobStatus::Failed;
        self.finished_at = Some(Utc::now());
        true
    }

    /// Cancels a non-terminal build from API/operator action.
    pub fn cancel(&mut self) -> bool {
        // Cancellation is a no-op once a build reached a terminal state.
        if matches!(
            self.status,
            JobStatus::Success | JobStatus::Failed | JobStatus::Canceled
        ) {
            return false;
        }

        self.status = JobStatus::Canceled;
        self.finished_at = Some(Utc::now());
        true
    }

    /// Requeues a running build back to pending for retry/reclaim flows.
    pub fn requeue_from_running(&mut self) -> bool {
        // Requeue resets execution timestamps so retries/stale reclaims look like fresh attempts.
        if self.status != JobStatus::Running {
            return false;
        }

        self.status = JobStatus::Pending;
        self.started_at = None;
        self.finished_at = None;
        true
    }

    /// Appends a human-readable log line to build execution history.
    pub fn append_log(&mut self, line: impl Into<String>) {
        self.logs.push(line.into());
    }
}

#[cfg(test)]
mod tests;
