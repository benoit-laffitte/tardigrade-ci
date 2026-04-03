mod assets;
mod handlers;

pub use assets::{APP_JS, INDEX_HTML, STYLES_CSS, TARDIGRADE_LOGO_PNG};
pub use handlers::{app_js, index, styles_css, tardigrade_logo_png};

#[cfg(test)]
mod tests;
