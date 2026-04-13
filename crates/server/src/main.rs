use anyhow::{Result, anyhow};
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
use dashboard::{WEB_ROOT_ENV_VAR, mount_dashboard_assets, resolve_web_root};
use runtime::{FILE_BACKED_PROD_DEPRECATION_TARGET, shutdown_signal};

struct ScmConfig {
    is_polling_enabled: bool, // true or false
    polling_check_interval: u64, // in seconds
}

impl ScmConfig {
    fn load() -> Self {
        let is_polling_enabled = std::env::var("TARDIGRADE_SCM_POLLING_ENABLED")
            .ok()
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "True"))
            .unwrap_or(false);
        
        let polling_check_interval = std::env::var("TARDIGRADE_SCM_POLLING_CHECK_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5);

        Self {
            is_polling_enabled,
            polling_check_interval,
        }
    }
}

struct QueueConfig {
    redis_url: Option<String>,
    redis_prefix: String,
    queue_file: Option<String>,
}

impl QueueConfig {
    fn load() -> Self {
        let redis_prefix = std::env::var("TARDIGRADE_REDIS_PREFIX")
                          .unwrap_or_else(|_| "tardigrade".to_string());
        let redis_url = std::env::var("TARDIGRADE_REDIS_URL").ok();
        let queue_file = std::env::var("TARDIGRADE_QUEUE_FILE").ok();
        
        Self {
            redis_url,
            redis_prefix,
            queue_file,
        }
    }
}

struct AppConfig {
    config_file: String,
    service_name: String,
    bind_address: String,
    has_embedded_worker: bool,
    database_url: Option<String>,
    scm: ScmConfig,
    queue: QueueConfig,
}

impl AppConfig {
    fn load() -> Result<Self> {

        let service_name = std::env::var("TARDIGRADE_SERVICE_NAME")
            .unwrap_or_else(|_| "tardigrade-ci".to_string());
        
        let config_file = std::env::var("TARDIGRADE_CONFIG_FILE")
            .unwrap_or_else(|_| "config/example.toml".to_string());
    
        let bind_address = std::env::var("TARDIGRADE_BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        
        let has_embedded_worker = std::env::var("TARDIGRADE_EMBEDDED_WORKER")
            .ok()
            .map(|v| !matches!(v.as_str(), "0" | "false" | "FALSE" | "False"))
            .unwrap_or(true);
        
        let database_url = std::env::var("TARDIGRADE_DATABASE_URL").ok();
 
        let scm = ScmConfig::load();
        let queue = QueueConfig::load();

        Ok(Self {
            config_file,
            service_name,
            bind_address,
            has_embedded_worker,
            database_url,
            scm,
            queue,
        })
    }
}

/// Builds the storage backend selected by runtime mode.
async fn build_storage(
    runtime_mode: RuntimeMode,
    database_url: Option<&str>,
) -> Result<Arc<dyn Storage + Send + Sync>> {
    let storage: Arc<dyn Storage + Send + Sync> = match runtime_mode {
        RuntimeMode::Prod => {
            let database_url = database_url
                .ok_or_else(|| anyhow!("prod mode requires TARDIGRADE_DATABASE_URL"))?;
            info!("using postgres-backed storage (prod mode)");
            Arc::new(PostgresStorage::connect(database_url).await?)
        }
        RuntimeMode::Dev => match database_url {
            // Dev mode keeps optional postgres for parity testing, defaulting to in-memory.
            Some(database_url) => {
                info!("using postgres-backed storage (dev mode)");
                Arc::new(PostgresStorage::connect(database_url).await?)
            }
            None => Arc::new(InMemoryStorage::default()),
        },
    };

    Ok(storage)
}

/// Builds the scheduler backend selected by runtime mode.
fn build_scheduler(
    runtime_mode: RuntimeMode,
    redis_url: Option<&str>,
    redis_prefix: &str,
) -> Result<Arc<dyn tardigrade_scheduler::Scheduler + Send + Sync>> {
    let scheduler: Arc<dyn tardigrade_scheduler::Scheduler + Send + Sync> = match runtime_mode {
        RuntimeMode::Prod => {
            let redis_url = redis_url.ok_or_else(|| anyhow!("prod mode requires TARDIGRADE_REDIS_URL"))?;
            info!(redis_prefix = %redis_prefix, "using redis-backed scheduler (prod mode)");
            Arc::new(RedisScheduler::open(redis_url, redis_prefix)?)
        }
        RuntimeMode::Dev => match redis_url {
            // Dev mode fallback chain is intentionally Redis -> in-memory.
            Some(redis_url) => {
                info!(redis_prefix = %redis_prefix, "using redis-backed scheduler (dev mode)");
                Arc::new(RedisScheduler::open(redis_url, redis_prefix)?)
            }
            None => Arc::new(tardigrade_scheduler::InMemoryScheduler::default()),
        },
    };

    Ok(scheduler)
}



/// Boots API server, selects configured backends, and serves HTTP routes.
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logs and let RUST_LOG override defaults.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Read runtime configuration from environment with safe defaults for local runs.
    let app_config = AppConfig::load()?;
    let AppConfig {
        config_file,
        service_name,
        bind_address,
        has_embedded_worker,
        database_url,
        scm,
        queue,
    } = app_config;

    let runtime_mode = load_runtime_mode_from_config(&config_file)?;
    let web_root = resolve_web_root();

    info!(config_file = %config_file, runtime_mode = ?runtime_mode, "runtime mode loaded");
    info!(
        web_root = %web_root.display(),
        web_root_env_var = WEB_ROOT_ENV_VAR,
        "dashboard web asset root resolved"
    );
    let queue_file = queue.queue_file.as_deref();
    if let Some(path) = queue_file {
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

    // Share one storage backend across request handlers through an Arc trait object.
    let storage = build_storage(runtime_mode, database_url.as_deref()).await?;

    // Same pattern for the scheduler: runtime decides concrete impl, API keeps trait abstraction.
    let redis_prefix = queue.redis_prefix.as_str();
    let redis_url = queue.redis_url.as_deref();
    let scheduler = build_scheduler(runtime_mode, redis_url, redis_prefix)?;
    let run_embedded_worker = has_embedded_worker;
    let state = ApiState::with_components_and_mode(
        service_name.clone(),
        storage,
        scheduler,
        run_embedded_worker,
    );
    let scm_polling_enabled = scm.is_polling_enabled;
    let scm_polling_check_secs = scm.polling_check_interval;
    if scm_polling_enabled {
        state.start_scm_polling_loop(Duration::from_secs(scm_polling_check_secs));
        info!(scm_polling_check_secs, "SCM polling loop enabled");
    }

    let router = mount_dashboard_assets(build_router(state));

    // Bind socket first, then hand listener to Axum for graceful shutdown support.
    let bind_addr = bind_address;
    let listener = TcpListener::bind(&bind_addr).await?;
    info!(bind_addr = %bind_addr, run_embedded_worker, "server listening");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
