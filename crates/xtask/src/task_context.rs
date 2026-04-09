use anyhow::{Result, anyhow};
use std::env;
use std::path::PathBuf;

/// Holds resolved workspace paths for task execution.
pub(crate) struct TaskContext {
    /// Absolute path to dashboard workspace used by npm tasks.
    pub(crate) dashboard_dir: PathBuf,
}

/// Resolves workspace and dashboard directories from crate location.
pub(crate) fn resolve_context() -> Result<TaskContext> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .ok_or_else(|| anyhow!("cannot resolve workspace root"))?
        .to_path_buf();

    let dashboard_dir = workspace_root.join("dashboard");
    if !dashboard_dir.exists() {
        return Err(anyhow!(
            "dashboard directory does not exist: {}",
            dashboard_dir.display()
        ));
    }

    Ok(TaskContext { dashboard_dir })
}
