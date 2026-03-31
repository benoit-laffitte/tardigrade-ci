use super::{
    BuildStrategyMetadata, RuntimeMetadata, TechnologyLanguage, TechnologyProfile,
    built_in_technology_profiles,
};

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
