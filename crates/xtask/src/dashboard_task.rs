use anyhow::{Context, Result, anyhow};
use std::process::{Command, Stdio};

use crate::task_context::TaskContext;

/// Executes one dashboard action using npm under controlled env.
pub(crate) fn run_dashboard_task(context: &TaskContext, action: &str) -> Result<()> {
    match action {
        "install" => run_npm(context, "install"),
        "lint" => run_npm(context, "run lint"),
        "build" => run_npm(context, "run build"),
        "dev" => run_npm(context, "run dev"),
        "all" => {
            run_npm(context, "install")?;
            run_npm(context, "run lint")?;
            run_npm(context, "run build")
        }
        _ => Err(anyhow!("unknown dashboard action: {action}")),
    }
}

/// Runs one npm command while bypassing user proxy and Nexus-like overrides.
fn run_npm(context: &TaskContext, npm_args: &str) -> Result<()> {
    let null_config = if cfg!(windows) { "NUL" } else { "/dev/null" };

    let status = Command::new("npm")
        .args(npm_args.split_whitespace())
        .current_dir(&context.dashboard_dir)
        .env_remove("https_proxy")
        .env_remove("http_proxy")
        .env_remove("HTTPS_PROXY")
        .env_remove("HTTP_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("NO_PROXY")
        .env_remove("no_proxy")
        .env_remove("PXY_FAB_FONC")
        .env("npm_config_userconfig", null_config)
        .env("npm_config_registry", "https://registry.npmjs.org/")
        .env("npm_config_proxy", "")
        .env("npm_config_https_proxy", "")
        .env("npm_config_strict_ssl", "false")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to run npm {npm_args}"))?;

    if !status.success() {
        return Err(anyhow!(
            "npm command failed with status {status}: npm {npm_args}"
        ));
    }

    Ok(())
}
