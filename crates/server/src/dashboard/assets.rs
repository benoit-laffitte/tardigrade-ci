use std::path::PathBuf;

/// Environment variable used to override the dashboard asset root.
pub const WEB_ROOT_ENV_VAR: &str = "TARDIGRADE_WEB_ROOT";

/// Resolves the dashboard asset root from env or uses the canonical target/public location.
pub fn resolve_web_root() -> PathBuf {
    if let Ok(explicit_root) = std::env::var(WEB_ROOT_ENV_VAR) {
        return PathBuf::from(explicit_root);
    }

    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(|path| path.parent())
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| crate_root.clone());

    // target/public is the canonical output of dashboard/vite.config.ts.
    workspace_root.join("target").join("public")
}
