use axum::Router;
use axum::routing::get_service;
use tower_http::services::ServeDir;

use super::assets::resolve_web_root;

/// Mounts dashboard web resources as a directory-backed fallback service.
pub fn mount_dashboard_assets(router: Router) -> Router {
    let service = ServeDir::new(resolve_web_root()).append_index_html_on_directories(true);

    router.fallback_service(get_service(service))
}
