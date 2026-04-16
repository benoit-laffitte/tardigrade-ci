use std::path::PathBuf;

/// Resolves the dashboard asset root from TOML config or canonical target/public location.
pub fn resolve_web_root(configured_root: Option<&str>) -> PathBuf {
    if let Some(explicit_root) = configured_root {
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
