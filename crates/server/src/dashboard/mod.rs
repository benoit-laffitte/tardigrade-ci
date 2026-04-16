mod assets;
mod service;

pub use self::{assets::resolve_web_root, service::mount_dashboard_assets};

#[cfg(test)]
mod tests;
