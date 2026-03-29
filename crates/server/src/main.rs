use anyhow::Result;
use axum::{
    response::{Html, IntoResponse},
    routing::get,
};
use tardigrade_api::{ApiState, build_router};
use tardigrade_scheduler::{FileBackedScheduler, RedisScheduler};
use tardigrade_storage::{InMemoryStorage, PostgresStorage, Storage};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Embedded dashboard main html payload.
const INDEX_HTML: &str = include_str!("../static/index.html");
/// Embedded dashboard javascript payload.
const APP_JS: &str = include_str!("../static/app.js");
/// Embedded dashboard stylesheet payload.
const STYLES_CSS: &str = include_str!("../static/styles.css");
/// Embedded dashboard logo payload.
const TARDIGRADE_LOGO_PNG: &[u8] = include_bytes!("../static/tardigrade-logo.png");

/// Boots API server, selects configured backends, and serves HTTP routes.
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let service_name = std::env::var("TARDIGRADE_SERVICE_NAME")
        .unwrap_or_else(|_| "tardigrade-ci".to_string());
    let bind_addr = std::env::var("TARDIGRADE_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let run_embedded_worker = std::env::var("TARDIGRADE_EMBEDDED_WORKER")
        .ok()
        .map(|v| !matches!(v.as_str(), "0" | "false" | "FALSE" | "False"))
        .unwrap_or(true);
    let redis_prefix = std::env::var("TARDIGRADE_REDIS_PREFIX")
        .unwrap_or_else(|_| "tardigrade".to_string());
    let storage: Arc<dyn Storage + Send + Sync> = match std::env::var("TARDIGRADE_DATABASE_URL") {
        // Prefer durable storage when configured, fallback to in-memory for bootstrap/dev.
        Ok(database_url) => {
            info!("using postgres-backed storage");
            Arc::new(PostgresStorage::connect(&database_url).await?)
        }
        Err(_) => Arc::new(InMemoryStorage::default()),
    };

    let scheduler: Arc<dyn tardigrade_scheduler::Scheduler + Send + Sync> = match std::env::var("TARDIGRADE_REDIS_URL") {
        // Scheduler fallback chain: Redis -> file-backed -> in-memory.
        Ok(redis_url) => {
            info!(redis_url = %redis_url, redis_prefix = %redis_prefix, "using redis-backed scheduler");
            Arc::new(RedisScheduler::open(&redis_url, &redis_prefix)?)
        }
        Err(_) => match std::env::var("TARDIGRADE_QUEUE_FILE") {
            Ok(path) => {
                info!(queue_file = %path, "using file-backed scheduler");
                Arc::new(FileBackedScheduler::open(path)?)
            }
            Err(_) => Arc::new(tardigrade_scheduler::InMemoryScheduler::default()),
        },
    };
    let state = ApiState::with_components_and_mode(service_name.clone(), storage, scheduler, run_embedded_worker);
    let router = build_router(state)
        .route("/", get(index))
        .route("/app.js", get(app_js))
        .route("/styles.css", get(styles_css))
        .route("/tardigrade-logo.png", get(tardigrade_logo_png));

    let listener = TcpListener::bind(&bind_addr).await?;
    info!(bind_addr = %bind_addr, run_embedded_worker, "server listening");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Serves dashboard index HTML.
async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

/// Serves dashboard javascript with explicit content type.
async fn app_js() -> impl IntoResponse {
    (
        [("content-type", "application/javascript; charset=utf-8")],
        APP_JS,
    )
}

/// Serves dashboard stylesheet with explicit content type.
async fn styles_css() -> impl IntoResponse {
    ([("content-type", "text/css; charset=utf-8")], STYLES_CSS)
}

/// Serves dashboard logo with explicit content type.
async fn tardigrade_logo_png() -> impl IntoResponse {
    ([("content-type", "image/png")], TARDIGRADE_LOGO_PNG)
}

/// Waits for termination signals and lets server shut down gracefully.
async fn shutdown_signal() {
    // Graceful shutdown lets in-flight requests complete before process exit.
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut stream) = signal(SignalKind::terminate()) {
            stream.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
