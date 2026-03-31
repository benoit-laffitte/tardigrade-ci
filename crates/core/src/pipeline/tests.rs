use super::{PipelineDefinition, PipelineDslError, PipelineRetryPolicy, PipelineStage, PipelineStep};

#[test]
/// Ensures v1 constructor sets schema version and preserves stage ordering.
fn pipeline_definition_v1_sets_version_and_stages() {
    let build_step = PipelineStep::new(
        "build",
        "rust:1.94",
        vec![
            "cargo".to_string(),
            "build".to_string(),
            "--workspace".to_string(),
        ],
    );
    let test_step = PipelineStep::new(
        "test",
        "rust:1.94",
        vec![
            "cargo".to_string(),
            "test".to_string(),
            "--workspace".to_string(),
        ],
    );

    let pipeline = PipelineDefinition::v1(vec![
        PipelineStage::new("compile", vec![build_step]),
        PipelineStage::new("verify", vec![test_step]),
    ]);

    assert_eq!(pipeline.version, 1);
    assert_eq!(pipeline.stages.len(), 2);
    assert_eq!(pipeline.stages[0].name, "compile");
    assert_eq!(pipeline.stages[1].name, "verify");
}

#[test]
/// Ensures retry policy hooks can be attached to one pipeline step.
fn pipeline_step_accepts_retry_policy_hook() {
    let retry = PipelineRetryPolicy::new(3, 1500);
    let mut step = PipelineStep::new(
        "unit-tests",
        "rust:1.94",
        vec!["cargo".to_string(), "test".to_string(), "--lib".to_string()],
    );

    step.retry = Some(retry);

    let retry = step.retry.expect("retry policy exists");
    assert_eq!(retry.max_attempts, 3);
    assert_eq!(retry.backoff_ms, 1500);
}

#[test]
/// Ensures YAML pipeline input is parsed and validated into the schema model.
fn pipeline_definition_from_yaml_parses_valid_input() {
    let yaml = r#"
version: 1
stages:
  - name: compile
    steps:
      - name: cargo-build
        image: "rust:1.94"
        command:
          - cargo
          - build
          - --workspace
        env:
          RUSTFLAGS: "-D warnings"
  - name: verify
    steps:
      - name: cargo-test
        image: "rust:1.94"
        command:
          - cargo
          - test
          - --workspace
        retry:
          max_attempts: 3
          backoff_ms: 1000
"#;

    let pipeline = PipelineDefinition::from_yaml_str(yaml).expect("pipeline should parse");
    assert_eq!(pipeline.version, 1);
    assert_eq!(pipeline.stages.len(), 2);
    assert_eq!(pipeline.stages[0].steps[0].name, "cargo-build");
    assert_eq!(
        pipeline.stages[1].steps[0]
            .retry
            .as_ref()
            .map(|retry| retry.max_attempts),
        Some(3)
    );
}

#[test]
/// Ensures YAML parser reports malformed documents as parsing errors.
fn pipeline_definition_from_yaml_rejects_invalid_yaml() {
    let malformed_yaml = "version: [1\nstages: []";

    let error = PipelineDefinition::from_yaml_str(malformed_yaml)
        .expect_err("malformed YAML should return an error");
    assert!(matches!(error, PipelineDslError::Yaml(_)));
}

#[test]
/// Ensures structural validator reports all high-signal schema violations.
fn pipeline_definition_validate_reports_structural_issues() {
    let yaml = r#"
version: 2
stages:
  - name: ""
    steps:
      - name: ""
        image: ""
        command: []
        retry:
          max_attempts: 0
          backoff_ms: 0
  - name: compile
    steps:
      - name: build
        image: "rust:1.94"
        command:
          - cargo
  - name: compile
    steps:
      - name: build
        image: "rust:1.94"
        command:
          - cargo
      - name: build
        image: "rust:1.94"
        command:
          - ""
"#;

    let error = PipelineDefinition::from_yaml_str(yaml)
        .expect_err("invalid structure should return validation issues");

    match error {
        PipelineDslError::Validation(issues) => {
            assert!(issues.iter().any(|issue| issue.field == "version"));
            assert!(issues.iter().any(|issue| issue.field == "stages[0].name"));
            assert!(
                issues
                    .iter()
                    .any(|issue| issue.field == "stages[0].steps[0].image")
            );
            assert!(
                issues
                    .iter()
                    .any(|issue| issue.field == "stages[0].steps[0].retry.max_attempts")
            );
            assert!(
                issues
                    .iter()
                    .any(|issue| issue.field == "stages[0].steps[0].retry.backoff_ms")
            );
            assert!(issues.iter().any(
                |issue| issue.field == "stages[2].name" && issue.message.contains("duplicate")
            ));
            assert!(issues.iter().any(|issue| {
                issue.field == "stages[2].steps[1].name" && issue.message.contains("duplicate")
            }));
            assert!(
                issues
                    .iter()
                    .any(|issue| issue.field == "stages[2].steps[1].command[0]")
            );
        }
        PipelineDslError::Yaml(message) => {
            panic!("expected validation errors, got YAML error: {message}")
        }
    }
}

#[test]
/// Ensures one pipeline can mix Rust, Python, and Java execution steps.
fn pipeline_definition_supports_multi_technology_stages() {
    let yaml = r#"
version: 1
stages:
  - name: rust-build
    steps:
      - name: compile
        image: "rust:1.94"
        command:
          - cargo
          - build
          - --workspace

  - name: python-tests
    steps:
      - name: pytest
        image: "python:3.12"
        command:
          - pytest
          - -q

  - name: java-tests
    steps:
      - name: maven-test
        image: "maven:3.9-eclipse-temurin-21"
        command:
          - mvn
          - -B
          - test
"#;

    let pipeline =
        PipelineDefinition::from_yaml_str(yaml).expect("mixed stack pipeline should parse");

    assert_eq!(pipeline.stages.len(), 3);
    assert_eq!(pipeline.stages[0].steps[0].image, "rust:1.94");
    assert_eq!(pipeline.stages[1].steps[0].image, "python:3.12");
    assert_eq!(
        pipeline.stages[2].steps[0].image,
        "maven:3.9-eclipse-temurin-21"
    );
}

#[test]
/// Ensures non-blocking hints detect common technology-specific CI pitfalls.
fn pipeline_validation_hints_detect_common_pitfalls() {
    let yaml = r#"
version: 1
stages:
  - name: rust
    steps:
      - name: cargo-build
        image: "rust:1.94"
        command: ["cargo", "build", "--workspace"]
  - name: python
    steps:
      - name: pip-install
        image: "python:3.12"
        command: ["pip", "install", "pytest"]
  - name: java
    steps:
      - name: maven-test
        image: "maven:3.9-eclipse-temurin-21"
        command: ["mvn", "test"]
  - name: node
    steps:
      - name: npm-install
        image: "node:20-bookworm"
        command: ["npm", "install"]
  - name: go
    steps:
      - name: go-test
        image: "golang:1.24-bookworm"
        command: ["go", "test"]
"#;

    let pipeline = PipelineDefinition::from_yaml_str(yaml).expect("pipeline should parse");
    let hints = pipeline.validation_hints();

    assert!(hints.iter().any(
        |hint| hint.message.contains("--locked") && hint.field == "stages[0].steps[0].command"
    ));
    assert!(
        hints
            .iter()
            .any(|hint| hint.message.contains("requirements")
                && hint.field == "stages[1].steps[0].command")
    );
    assert!(
        hints
            .iter()
            .any(|hint| hint.message.contains("-B") && hint.field == "stages[2].steps[0].command")
    );
    assert!(
        hints
            .iter()
            .any(|hint| hint.message.contains("npm ci")
                && hint.field == "stages[3].steps[0].command")
    );
    assert!(
        hints.iter().any(
            |hint| hint.message.contains("./...") && hint.field == "stages[4].steps[0].command"
        )
    );
}

#[test]
/// Ensures best-practice commands do not emit non-blocking hints.
fn pipeline_validation_hints_are_empty_for_best_practices() {
    let yaml = r#"
version: 1
stages:
  - name: rust
    steps:
      - name: cargo-build
        image: "rust:1.94"
        command: ["cargo", "build", "--workspace", "--locked"]
  - name: python
    steps:
      - name: pip-install
        image: "python:3.12"
        command: ["pip", "install", "-r", "requirements.txt"]
  - name: java
    steps:
      - name: maven-test
        image: "maven:3.9-eclipse-temurin-21"
        command: ["mvn", "-B", "test"]
  - name: node
    steps:
      - name: npm-install
        image: "node:20-bookworm"
        command: ["npm", "ci"]
  - name: go
    steps:
      - name: go-test
        image: "golang:1.24-bookworm"
        command: ["go", "test", "./..."]
"#;

    let pipeline = PipelineDefinition::from_yaml_str(yaml).expect("pipeline should parse");
    let hints = pipeline.validation_hints();

    assert!(hints.is_empty());
}
