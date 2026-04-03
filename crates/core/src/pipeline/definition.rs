use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{
    PipelineDslError, PipelineStage, PipelineStep, PipelineValidationHint, PipelineValidationIssue,
};

/// Versioned pipeline schema executed by workers for one job definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineDefinition {
    pub version: u32,
    pub stages: Vec<PipelineStage>,
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

        let mut stage_names = BTreeSet::new();
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

            let mut step_names = BTreeSet::new();
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

    /// Returns non-blocking recommendations for common technology-specific pitfalls.
    pub fn validation_hints(&self) -> Vec<PipelineValidationHint> {
        let mut hints = Vec::new();

        for (stage_idx, stage) in self.stages.iter().enumerate() {
            for (step_idx, step) in stage.steps.iter().enumerate() {
                let command_field = format!("stages[{stage_idx}].steps[{step_idx}].command");

                if is_rust_step(step)
                    && step_command_starts_with(step, &["cargo", "build"])
                    && !step_command_contains(step, "--locked")
                {
                    hints.push(PipelineValidationHint {
                        field: command_field.clone(),
                        message: "Rust build should include --locked for deterministic dependency resolution".to_string(),
                    });
                }

                if is_python_step(step)
                    && (step_command_starts_with(step, &["pip", "install"])
                        || step_uses_python_pip_install(step))
                    && !step_command_contains(step, "-r")
                {
                    hints.push(PipelineValidationHint {
                        field: command_field.clone(),
                        message:
                            "Python dependency install should prefer requirements lock file via -r"
                                .to_string(),
                    });
                }

                if is_java_step(step)
                    && step_command_starts_with(step, &["mvn"])
                    && !step_command_contains(step, "-B")
                {
                    hints.push(PipelineValidationHint {
                        field: command_field.clone(),
                        message: "Maven command should include -B for non-interactive CI execution"
                            .to_string(),
                    });
                }

                if is_node_step(step) && step_command_starts_with(step, &["npm", "install"]) {
                    hints.push(PipelineValidationHint {
                        field: command_field.clone(),
                        message:
                            "Node dependency install should prefer npm ci for reproducible installs"
                                .to_string(),
                    });
                }

                if is_go_step(step)
                    && step_command_starts_with(step, &["go", "test"])
                    && !step_command_contains(step, "./...")
                {
                    hints.push(PipelineValidationHint {
                        field: command_field,
                        message: "Go test should usually target ./... to cover all modules"
                            .to_string(),
                    });
                }
            }
        }

        hints
    }
}

/// Detects whether one step is likely to run Rust toolchain commands.
fn is_rust_step(step: &PipelineStep) -> bool {
    step.image.contains("rust") || step.command.first().is_some_and(|token| token == "cargo")
}

/// Detects whether one step is likely to run Python toolchain commands.
fn is_python_step(step: &PipelineStep) -> bool {
    step.image.contains("python")
        || step
            .command
            .first()
            .is_some_and(|token| token == "pip" || token == "python" || token == "pytest")
}

/// Detects whether one step is likely to run Java/Maven toolchain commands.
fn is_java_step(step: &PipelineStep) -> bool {
    step.image.contains("maven") || step.command.first().is_some_and(|token| token == "mvn")
}

/// Detects whether one step is likely to run Node.js/NPM commands.
fn is_node_step(step: &PipelineStep) -> bool {
    step.image.contains("node") || step.command.first().is_some_and(|token| token == "npm")
}

/// Detects whether one step is likely to run Go toolchain commands.
fn is_go_step(step: &PipelineStep) -> bool {
    step.image.contains("golang") || step.command.first().is_some_and(|token| token == "go")
}

/// Returns true when one command starts with the expected token sequence.
fn step_command_starts_with(step: &PipelineStep, expected_prefix: &[&str]) -> bool {
    step.command
        .iter()
        .take(expected_prefix.len())
        .map(String::as_str)
        .eq(expected_prefix.iter().copied())
}

/// Returns true when one token is present in command arguments.
fn step_command_contains(step: &PipelineStep, token: &str) -> bool {
    step.command.iter().any(|entry| entry == token)
}

/// Returns true when command is equivalent to `python -m pip install ...`.
fn step_uses_python_pip_install(step: &PipelineStep) -> bool {
    step.command.len() >= 4
        && step.command[0] == "python"
        && step.command[1] == "-m"
        && step.command[2] == "pip"
        && step.command[3] == "install"
}
