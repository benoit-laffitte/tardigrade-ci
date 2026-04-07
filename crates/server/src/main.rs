use anyhow::{Result, anyhow};
use axum::routing::get;
use std::sync::Arc;
use std::time::Duration;
use tardigrade_api::{ApiState, build_router};
use tardigrade_scheduler::RedisScheduler;
use tardigrade_storage::{InMemoryStorage, PostgresStorage, Storage};
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod config;
mod dashboard;
mod runtime;

use config::{RuntimeMode, load_runtime_mode_from_config};
use dashboard::{WEB_ROOT_ENV_VAR, app_js, index, resolve_web_root, styles_css, tardigrade_logo_png};
use runtime::{FILE_BACKED_PROD_DEPRECATION_TARGET, shutdown_signal};

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
    let scm_polling_enabled = std::env::var("TARDIGRADE_SCM_POLLING_ENABLED")
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "True"))
        .unwrap_or(false);
    let scm_polling_check_secs = std::env::var("TARDIGRADE_SCM_POLLING_CHECK_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(5);
    let database_url = std::env::var("TARDIGRADE_DATABASE_URL").ok();
    let redis_url = std::env::var("TARDIGRADE_REDIS_URL").ok();
    let queue_file = std::env::var("TARDIGRADE_QUEUE_FILE").ok();
    let web_root = resolve_web_root();

    info!(config_file = %config_file, runtime_mode = ?runtime_mode, "runtime mode loaded");
    info!(
        web_root = %web_root.display(),
        web_root_env_var = WEB_ROOT_ENV_VAR,
        "dashboard web asset root resolved"
    );

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

    if scm_polling_enabled {
        state.start_scm_polling_loop(Duration::from_secs(scm_polling_check_secs));
        info!(scm_polling_check_secs, "SCM polling loop enabled");
    }

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
