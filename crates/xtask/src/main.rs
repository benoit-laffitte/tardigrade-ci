use anyhow::{Context, Result, anyhow};
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Holds resolved workspace paths for task execution.
struct TaskContext {
    dashboard_dir: PathBuf,
}

/// Parses CLI arguments and dispatches to requested task.
fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let context = resolve_context()?;

    match args.as_slice() {
        [scope] if scope == "help" || scope == "--help" || scope == "-h" => {
            print_help();
            Ok(())
        }
        [scope] if scope == "dashboard" => run_dashboard_task(&context, "all"),
        [scope, action] if scope == "dashboard" => run_dashboard_task(&context, action),
        _ => {
            print_help();
            Err(anyhow!("invalid task arguments"))
        }
    }
}

/// Resolves workspace and dashboard directories from crate location.
fn resolve_context() -> Result<TaskContext> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .ok_or_else(|| anyhow!("cannot resolve workspace root"))?
        .to_path_buf();

    let dashboard_dir = workspace_root.join("crates/server/dashboard");
    if !dashboard_dir.exists() {
        return Err(anyhow!(
            "dashboard directory does not exist: {}",
            dashboard_dir.display()
        ));
    }

    Ok(TaskContext { dashboard_dir })
}

/// Prints CLI usage for available centralized tasks.
fn print_help() {
    println!("tardigrade-xtask");
    println!();
    println!("Usage:");
    println!("  cargo xtask dashboard [install|lint|build|dev|all]");
    println!();
    println!("Examples:");
    println!("  cargo xtask dashboard build");
    println!("  cargo dashboard-build");
}

/// Executes one dashboard action using npm under controlled env.
fn run_dashboard_task(context: &TaskContext, action: &str) -> Result<()> {
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
