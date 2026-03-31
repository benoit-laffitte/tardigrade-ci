use anyhow::{Result, anyhow};
use tardigrade_core::ScmProvider;

/// Maps SCM provider enum to compact persisted text representation.
pub(crate) fn scm_provider_to_str(provider: ScmProvider) -> &'static str {
    match provider {
        ScmProvider::Github => "github",
        ScmProvider::Gitlab => "gitlab",
    }
}

/// Parses persisted SCM provider text into enum value.
pub(crate) fn parse_scm_provider(raw: &str) -> Result<ScmProvider> {
    match raw {
        "github" => Ok(ScmProvider::Github),
        "gitlab" => Ok(ScmProvider::Gitlab),
        other => Err(anyhow!("unknown SCM provider in storage: {other}")),
    }
}
