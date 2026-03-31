use super::{
    BuildStrategyMetadata, RuntimeMetadata, TechnologyLanguage, TechnologyProfileValidationIssue,
};
use serde::{Deserialize, Serialize};

/// Versionable technology profile model used to bootstrap stack-specific pipeline defaults.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TechnologyProfile {
    pub id: String,
    pub display_name: String,
    pub language: TechnologyLanguage,
    pub runtime: RuntimeMetadata,
    pub strategy: BuildStrategyMetadata,
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
