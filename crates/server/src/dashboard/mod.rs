mod assets;
mod service;

pub use assets::{WEB_ROOT_ENV_VAR, resolve_web_root};
pub use service::mount_dashboard_assets;

#[cfg(test)]
mod tests;
