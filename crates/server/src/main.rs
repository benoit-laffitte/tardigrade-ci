use anyhow::{Context, Result, anyhow};
use axum::{
    response::{Html, IntoResponse},
    routing::get,
};
use serde::Deserialize;
use std::fs;
use std::sync::Arc;
use tardigrade_api::{ApiState, build_router};
use tardigrade_scheduler::RedisScheduler;
use tardigrade_storage::{InMemoryStorage, PostgresStorage, Storage};
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

/// Embedded dashboard main html payload.
const INDEX_HTML: &str = include_str!("../static/index.html");
/// Embedded dashboard javascript payload.
const APP_JS: &str = include_str!("../static/app.js");
/// Embedded dashboard stylesheet payload.
const STYLES_CSS: &str = include_str!("../static/styles.css");
/// Sunset target for removing any legacy queue-file flows outside dev mode.
const FILE_BACKED_PROD_DEPRECATION_TARGET: &str = "2026-09-30";

/// Runtime mode derived from configuration file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RuntimeMode {
    Dev,
    Prod,
}

impl Default for RuntimeMode {
    /// Defaults runtime mode to dev when configuration omits explicit value.
    fn default() -> Self {
        Self::Dev
    }
}

/// Top-level config file shape used by server bootstrap.
#[derive(Debug, Deserialize, Default)]
struct ServerConfigFile {
    runtime: Option<RuntimeSection>,
}

/// Runtime-specific configuration section.
#[derive(Debug, Deserialize)]
struct RuntimeSection {
    mode: RuntimeMode,
}

/// Parses runtime mode from TOML payload.
fn parse_runtime_mode_from_toml(raw: &str) -> Result<RuntimeMode> {
    let config: ServerConfigFile = toml::from_str(raw).context("parse TOML configuration")?;
    Ok(config
        .runtime
        .map(|runtime| runtime.mode)
        .unwrap_or_default())
}

/// Loads runtime mode from config file path, defaulting to dev when file is missing.
fn load_runtime_mode_from_config(path: &str) -> Result<RuntimeMode> {
    match fs::read_to_string(path) {
        Ok(raw) => parse_runtime_mode_from_toml(&raw),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            info!(config_path = %path, "config file not found, defaulting runtime mode to dev");
            Ok(RuntimeMode::Dev)
        }
        Err(err) => Err(err).with_context(|| format!("read config file at {path}")),
    }
}

/// Boots API server, selects configured backends, and serves HTTP routes.
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let service_name =
        std::env::var("TARDIGRADE_SERVICE_NAME").unwrap_or_else(|_| "tardigrade-ci".to_string());
    let config_file = std::env::var("TARDIGRADE_CONFIG_FILE")
        .unwrap_or_else(|_| "config/example.toml".to_string());
    let runtime_mode = load_runtime_mode_from_config(&config_file)?;
    let bind_addr =
        std::env::var("TARDIGRADE_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let run_embedded_worker = std::env::var("TARDIGRADE_EMBEDDED_WORKER")
        .ok()
        .map(|v| !matches!(v.as_str(), "0" | "false" | "FALSE" | "False"))
        .unwrap_or(true);
    let redis_prefix =
        std::env::var("TARDIGRADE_REDIS_PREFIX").unwrap_or_else(|_| "tardigrade".to_string());
    let database_url = std::env::var("TARDIGRADE_DATABASE_URL").ok();
    let redis_url = std::env::var("TARDIGRADE_REDIS_URL").ok();
    let queue_file = std::env::var("TARDIGRADE_QUEUE_FILE").ok();

    info!(config_file = %config_file, runtime_mode = ?runtime_mode, "runtime mode loaded");

    if let Some(path) = queue_file.as_deref() {
        if runtime_mode != RuntimeMode::Dev {
            warn!(
                queue_file = %path,
                sunset_target = FILE_BACKED_PROD_DEPRECATION_TARGET,
                "TARDIGRADE_QUEUE_FILE is deprecated outside dev mode and ignored",
            );
        } else {
            warn!(queue_file = %path, "TARDIGRADE_QUEUE_FILE is deprecated and ignored in dev mode");
        }
    }

    let storage: Arc<dyn Storage + Send + Sync> = match runtime_mode {
        RuntimeMode::Prod => {
            let database_url = database_url
                .as_deref()
                .ok_or_else(|| anyhow!("prod mode requires TARDIGRADE_DATABASE_URL"))?;
            info!("using postgres-backed storage (prod mode)");
            Arc::new(PostgresStorage::connect(database_url).await?)
        }
        RuntimeMode::Dev => match database_url.as_deref() {
            // Dev mode keeps optional postgres for parity testing, defaulting to in-memory.
            Some(database_url) => {
                info!("using postgres-backed storage (dev mode)");
                Arc::new(PostgresStorage::connect(database_url).await?)
            }
            None => Arc::new(InMemoryStorage::default()),
        },
    };

    let scheduler: Arc<dyn tardigrade_scheduler::Scheduler + Send + Sync> = match runtime_mode {
        RuntimeMode::Prod => {
            let redis_url = redis_url
                .as_deref()
                .ok_or_else(|| anyhow!("prod mode requires TARDIGRADE_REDIS_URL"))?;
            info!(redis_url = %redis_url, redis_prefix = %redis_prefix, "using redis-backed scheduler (prod mode)");
            Arc::new(RedisScheduler::open(redis_url, &redis_prefix)?)
        }
        RuntimeMode::Dev => match redis_url.as_deref() {
            // Dev mode fallback chain is intentionally Redis -> in-memory.
            Some(redis_url) => {
                info!(redis_url = %redis_url, redis_prefix = %redis_prefix, "using redis-backed scheduler (dev mode)");
                Arc::new(RedisScheduler::open(redis_url, &redis_prefix)?)
            }
            None => Arc::new(tardigrade_scheduler::InMemoryScheduler::default()),
        },
    };
    let state = ApiState::with_components_and_mode(
        service_name.clone(),
        storage,
        scheduler,
        run_embedded_worker,
    );
    let router = build_router(state)
        .route("/", get(index))
        .route("/app.js", get(app_js))
        .route("/styles.css", get(styles_css));

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

#[cfg(test)]
mod tests;
