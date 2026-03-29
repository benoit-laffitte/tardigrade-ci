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

/// Runtime metadata used to select container and shell behavior for one profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeMetadata {
    pub image: String,
    #[serde(default)]
    pub shell: Option<String>,
}

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

/// Versionable technology profile model used to bootstrap stack-specific pipeline defaults.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TechnologyProfile {
    pub id: String,
    pub display_name: String,
    pub language: TechnologyLanguage,
    pub runtime: RuntimeMetadata,
    pub strategy: BuildStrategyMetadata,
}

/// One actionable validation issue found in a technology profile model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TechnologyProfileValidationIssue {
    pub field: String,
    pub message: String,
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

impl TechnologyProfile {
    /// Creates one technology profile from language/runtime/build metadata.
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        language: TechnologyLanguage,
        runtime: RuntimeMetadata,
        strategy: BuildStrategyMetadata,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            language,
            runtime,
            strategy,
        }
    }

    /// Validates required profile metadata for deterministic catalog consumption.
    pub fn validate(&self) -> Result<(), Vec<TechnologyProfileValidationIssue>> {
        let mut issues = Vec::new();

        if self.id.trim().is_empty() {
            issues.push(TechnologyProfileValidationIssue {
                field: "id".to_string(),
                message: "profile id cannot be empty".to_string(),
            });
        }

        if self.display_name.trim().is_empty() {
            issues.push(TechnologyProfileValidationIssue {
                field: "display_name".to_string(),
                message: "display_name cannot be empty".to_string(),
            });
        }

        if self.runtime.image.trim().is_empty() {
            issues.push(TechnologyProfileValidationIssue {
                field: "runtime.image".to_string(),
                message: "runtime image cannot be empty".to_string(),
            });
        }

        if self
            .runtime
            .shell
            .as_ref()
            .is_some_and(|shell| shell.trim().is_empty())
        {
            issues.push(TechnologyProfileValidationIssue {
                field: "runtime.shell".to_string(),
                message: "runtime shell cannot be blank when provided".to_string(),
            });
        }

        if self.strategy.build.is_empty() && self.strategy.test.is_empty() {
            issues.push(TechnologyProfileValidationIssue {
                field: "strategy".to_string(),
                message: "at least one build or test command is required".to_string(),
            });
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

/// Returns the built-in technology profile catalog for first-class supported stacks.
pub fn built_in_technology_profiles() -> Vec<TechnologyProfile> {
    vec![
        TechnologyProfile::new(
            "rust",
            "Rust",
            TechnologyLanguage::Rust,
            RuntimeMetadata {
                image: "rust:1.94".to_string(),
                shell: Some("bash".to_string()),
            },
            BuildStrategyMetadata {
                install: vec!["cargo fetch".to_string()],
                build: vec!["cargo build --workspace".to_string()],
                test: vec!["cargo test --workspace".to_string()],
                package: vec!["cargo build --release".to_string()],
            },
        ),
        TechnologyProfile::new(
            "python",
            "Python",
            TechnologyLanguage::Python,
            RuntimeMetadata {
                image: "python:3.12".to_string(),
                shell: Some("bash".to_string()),
            },
            BuildStrategyMetadata {
                install: vec!["pip install -r requirements.txt".to_string()],
                build: Vec::new(),
                test: vec!["pytest -q".to_string()],
                package: vec!["python -m build".to_string()],
            },
        ),
        TechnologyProfile::new(
            "java",
            "Java",
            TechnologyLanguage::Java,
            RuntimeMetadata {
                image: "maven:3.9-eclipse-temurin-21".to_string(),
                shell: Some("bash".to_string()),
            },
            BuildStrategyMetadata {
                install: vec!["mvn -B -q -DskipTests dependency:go-offline".to_string()],
                build: vec!["mvn -B -DskipTests package".to_string()],
                test: vec!["mvn -B test".to_string()],
                package: vec!["mvn -B package".to_string()],
            },
        ),
        TechnologyProfile::new(
            "node",
            "Node.js",
            TechnologyLanguage::Node,
            RuntimeMetadata {
                image: "node:20-bookworm".to_string(),
                shell: Some("bash".to_string()),
            },
            BuildStrategyMetadata {
                install: vec!["npm ci".to_string()],
                build: vec!["npm run build".to_string()],
                test: vec!["npm test".to_string()],
                package: vec!["npm pack".to_string()],
            },
        ),
        TechnologyProfile::new(
            "go",
            "Go",
            TechnologyLanguage::Go,
            RuntimeMetadata {
                image: "golang:1.24-bookworm".to_string(),
                shell: Some("bash".to_string()),
            },
            BuildStrategyMetadata {
                install: vec!["go mod download".to_string()],
                build: vec!["go build ./...".to_string()],
                test: vec!["go test ./...".to_string()],
                package: vec!["go build -o app".to_string()],
            },
        ),
    ]
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
