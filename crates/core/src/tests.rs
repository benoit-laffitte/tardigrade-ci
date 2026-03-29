use super::{
    BuildStrategyMetadata, PipelineDefinition, PipelineDslError, PipelineRetryPolicy,
    PipelineStage, PipelineStep, RuntimeMetadata, TechnologyLanguage, TechnologyProfile,
    built_in_technology_profiles,
};

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
/// Ensures technology profile constructor preserves language, runtime, and strategy metadata.
fn technology_profile_new_sets_expected_fields() {
    let profile = TechnologyProfile::new(
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
    );

    assert_eq!(profile.id, "rust");
    assert_eq!(profile.display_name, "Rust");
    assert_eq!(profile.language, TechnologyLanguage::Rust);
    assert_eq!(profile.runtime.image, "rust:1.94");
    assert_eq!(profile.runtime.shell.as_deref(), Some("bash"));
    assert_eq!(profile.strategy.build.len(), 1);
    assert_eq!(profile.strategy.test.len(), 1);
}

#[test]
/// Ensures technology profile model remains serde-compatible for API/storage boundaries.
fn technology_profile_serialization_roundtrip_preserves_content() {
    let profile = TechnologyProfile::new(
        "python",
        "Python",
        TechnologyLanguage::Python,
        RuntimeMetadata {
            image: "python:3.12".to_string(),
            shell: None,
        },
        BuildStrategyMetadata {
            install: vec!["pip install -r requirements.txt".to_string()],
            build: Vec::new(),
            test: vec!["pytest -q".to_string()],
            package: Vec::new(),
        },
    );

    let serialized = serde_yaml::to_string(&profile).expect("technology profile should serialize");
    let restored: TechnologyProfile =
        serde_yaml::from_str(&serialized).expect("technology profile should deserialize");

    assert_eq!(restored, profile);
}

#[test]
/// Ensures technology profile validation reports empty identity/runtime/strategy fields.
fn technology_profile_validate_reports_required_fields() {
    let invalid_profile = TechnologyProfile::new(
        " ",
        "",
        TechnologyLanguage::Go,
        RuntimeMetadata {
            image: " ".to_string(),
            shell: Some(" ".to_string()),
        },
        BuildStrategyMetadata {
            install: Vec::new(),
            build: Vec::new(),
            test: Vec::new(),
            package: Vec::new(),
        },
    );

    let issues = invalid_profile
        .validate()
        .expect_err("invalid profile should return validation issues");

    assert!(issues.iter().any(|issue| issue.field == "id"));
    assert!(issues.iter().any(|issue| issue.field == "display_name"));
    assert!(issues.iter().any(|issue| issue.field == "runtime.image"));
    assert!(issues.iter().any(|issue| issue.field == "runtime.shell"));
    assert!(issues.iter().any(|issue| issue.field == "strategy"));
}

#[test]
/// Ensures built-in technology profile catalog exposes all first-class stacks.
fn built_in_technology_profiles_include_expected_languages() {
    let profiles = built_in_technology_profiles();

    assert_eq!(profiles.len(), 5);
    assert!(
        profiles
            .iter()
            .any(|profile| profile.language == TechnologyLanguage::Rust)
    );
    assert!(
        profiles
            .iter()
            .any(|profile| profile.language == TechnologyLanguage::Python)
    );
    assert!(
        profiles
            .iter()
            .any(|profile| profile.language == TechnologyLanguage::Java)
    );
    assert!(
        profiles
            .iter()
            .any(|profile| profile.language == TechnologyLanguage::Node)
    );
    assert!(
        profiles
            .iter()
            .any(|profile| profile.language == TechnologyLanguage::Go)
    );
}

#[test]
/// Ensures built-in catalog entries have unique ids and pass profile validation.
fn built_in_technology_profiles_are_unique_and_valid() {
    let profiles = built_in_technology_profiles();
    let mut ids = std::collections::BTreeSet::new();

    for profile in profiles {
        assert!(ids.insert(profile.id.clone()));
        assert!(profile.validate().is_ok());
    }
}
