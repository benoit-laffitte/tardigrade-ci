mod assets;
mod handlers;

pub use assets::{WEB_ROOT_ENV_VAR, resolve_web_root};
pub use handlers::{app_js, index, styles_css, tardigrade_logo_png};

#[cfg(test)]
mod tests;
