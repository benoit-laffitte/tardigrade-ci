use std::path::PathBuf;

/// Environment variable used to override the dashboard asset root.
pub const WEB_ROOT_ENV_VAR: &str = "TARDIGRADE_WEB_ROOT";

/// Resolves the dashboard asset root from env or defaults to the repository static folder.
pub fn resolve_web_root() -> PathBuf {
    std::env::var(WEB_ROOT_ENV_VAR)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static"))
}

/// Resolves one dashboard asset path under the configured dashboard root.
pub fn resolve_asset_path(file_name: &str) -> PathBuf {
    resolve_web_root().join(file_name)
}
