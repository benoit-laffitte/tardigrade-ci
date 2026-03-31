mod assets;
mod handlers;

pub use assets::{APP_JS, INDEX_HTML, STYLES_CSS};
pub use handlers::{app_js, index, styles_css};

#[cfg(test)]
mod tests;
