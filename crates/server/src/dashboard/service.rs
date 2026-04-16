use axum::Router;
use axum::routing::get_service;
use std::path::PathBuf;
use tower_http::services::ServeDir;

/// Mounts dashboard web resources as a directory-backed fallback service.
pub fn mount_dashboard_assets(router: Router, web_root: PathBuf) -> Router {
    let service = ServeDir::new(web_root).append_index_html_on_directories(true);

    router.fallback_service(get_service(service))
}
