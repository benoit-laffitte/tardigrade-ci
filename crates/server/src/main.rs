use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::Duration;
use tardigrade_api::{ApiState, build_router};
use tardigrade_scheduler::{
    FileBackedScheduler, InMemoryScheduler, PostgresScheduler, RedisScheduler,
};
use tardigrade_storage::{InMemoryStorage, PostgresStorage, Storage};
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod config;
mod dashboard;
mod runtime;
mod webhook_adapter;

#[cfg(test)]
mod webhook_adapter_tests;

use config::{RuntimeMode, load_runtime_mode_from_config};
use dashboard::{WEB_ROOT_ENV_VAR, mount_dashboard_assets, resolve_web_root};
use runtime::shutdown_signal;
use webhook_adapter::mount_webhook_adapter;

/// Parses an environment variable as bool and warns when the value is invalid.
fn parse_env_bool(
    var_name: &str,
    default: bool,
    true_values: &[&str],
    false_values: &[&str],
) -> bool {
    match std::env::var(var_name) {
        Ok(raw) if true_values.contains(&raw.as_str()) => true,
        Ok(raw) if false_values.contains(&raw.as_str()) => false,
        Ok(raw) => {
            warn!(env_var = var_name, value = %raw, default, "invalid boolean env value; using default");
            default
        }
        Err(_) => default,
    }
}

/// Parses an environment variable as u64 and warns when parsing fails.
fn parse_env_u64(var_name: &str, default: u64) -> u64 {
    match std::env::var(var_name) {
        Ok(raw) => match raw.parse::<u64>() {
            Ok(value) => value,
            Err(_) => {
                warn!(env_var = var_name, value = %raw, default, "invalid u64 env value; using default");
                default
            }
        },
        Err(_) => default,
    }
}

/// Holds SCM polling-related configuration derived from environment variables.
struct ScmConfig {
    is_polling_enabled: bool,    // true or false
    polling_check_interval: u64, // in seconds
}

impl ScmConfig {
    /// Loads SCM polling settings with defaults suitable for local runs.
    fn load() -> Self {
        let is_polling_enabled = parse_env_bool(
            "TARDIGRADE_SCM_POLLING_ENABLED",
            false,
            &["1", "true", "TRUE", "True"],
            &["0", "false", "FALSE", "False"],
        );

        let polling_check_interval = parse_env_u64("TARDIGRADE_SCM_POLLING_CHECK_SECS", 5);

        Self {
            is_polling_enabled,
            polling_check_interval,
        }
    }
}

/// Holds queue and scheduler-related configuration derived from environment variables.
struct QueueConfig {
    redis_url: Option<String>,
    redis_prefix: String,
    queue_file: Option<String>,
    scheduler_backend: Option<String>,
    scheduler_database_url: Option<String>,
    scheduler_namespace: String,
}

impl QueueConfig {
    /// Loads queue settings while keeping optional Redis/file-backed inputs explicit.
    fn load() -> Self {
        let redis_prefix =
            std::env::var("TARDIGRADE_REDIS_PREFIX").unwrap_or_else(|_| "tardigrade".to_string());
        let redis_url = std::env::var("TARDIGRADE_REDIS_URL").ok();
        let queue_file = std::env::var("TARDIGRADE_QUEUE_FILE").ok();
        let scheduler_backend = std::env::var("TARDIGRADE_SCHEDULER_BACKEND").ok();
        let scheduler_database_url = std::env::var("TARDIGRADE_SCHEDULER_DATABASE_URL").ok();
        let scheduler_namespace = std::env::var("TARDIGRADE_SCHEDULER_NAMESPACE")
            .unwrap_or_else(|_| "tardigrade".to_string());

        Self {
            redis_url,
            redis_prefix,
            queue_file,
            scheduler_backend,
            scheduler_database_url,
            scheduler_namespace,
        }
    }
}

/// Enumerates supported scheduler backend implementations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SchedulerBackend {
    InMemory,
    File,
    Redis,
    Postgres,
}

impl SchedulerBackend {
    /// Parses backend identifiers from environment variables.
    fn parse(raw: &str) -> Option<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "in-memory" | "in_memory" | "inmemory" => Some(Self::InMemory),
            "file" => Some(Self::File),
            "redis" => Some(Self::Redis),
            "postgres" | "postgresql" => Some(Self::Postgres),
            _ => None,
        }
    }
}

/// Aggregates all runtime configuration needed to boot the server process.
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
    /// Loads the full server configuration from environment variables and defaults.
    fn load() -> Result<Self> {
        let service_name = std::env::var("TARDIGRADE_SERVICE_NAME")
            .unwrap_or_else(|_| "tardigrade-ci".to_string());

        let config_file = std::env::var("TARDIGRADE_CONFIG_FILE")
            .unwrap_or_else(|_| "config/example.toml".to_string());

        let bind_address =
            std::env::var("TARDIGRADE_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

        let has_embedded_worker = parse_env_bool(
            "TARDIGRADE_EMBEDDED_WORKER",
            true,
            &["1", "true", "TRUE", "True"],
            &["0", "false", "FALSE", "False"],
        );

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
    queue: &QueueConfig,
    storage_database_url: Option<&str>,
) -> Result<(
    Arc<dyn tardigrade_scheduler::Scheduler + Send + Sync>,
    SchedulerBackend,
)> {
    let configured_backend = queue
        .scheduler_backend
        .as_deref()
        .map(|raw| {
            SchedulerBackend::parse(raw).ok_or_else(|| {
                anyhow!(
                    "invalid TARDIGRADE_SCHEDULER_BACKEND value: {raw} (expected one of: in-memory, file, redis, postgres)"
                )
            })
        })
        .transpose()?;

    let selected_backend = match configured_backend {
        Some(backend) => backend,
        None => match runtime_mode {
            RuntimeMode::Prod => SchedulerBackend::Redis,
            RuntimeMode::Dev => {
                if queue.redis_url.is_some() {
                    SchedulerBackend::Redis
                } else {
                    SchedulerBackend::InMemory
                }
            }
        },
    };

    let scheduler: Arc<dyn tardigrade_scheduler::Scheduler + Send + Sync> = match selected_backend {
        SchedulerBackend::InMemory => {
            info!("using in-memory scheduler");
            Arc::new(InMemoryScheduler::default())
        }
        SchedulerBackend::File => {
            let queue_file = queue
                .queue_file
                .as_deref()
                .ok_or_else(|| anyhow!("file scheduler requires TARDIGRADE_QUEUE_FILE"))?;
            info!(queue_file = %queue_file, "using file-backed scheduler");
            Arc::new(FileBackedScheduler::open(queue_file)?)
        }
        SchedulerBackend::Redis => {
            let redis_url = queue
                .redis_url
                .as_deref()
                .ok_or_else(|| anyhow!("redis scheduler requires TARDIGRADE_REDIS_URL"))?;
            let redis_prefix = queue.redis_prefix.as_str();
            info!(redis_prefix = %redis_prefix, "using redis-backed scheduler");
            Arc::new(RedisScheduler::open(redis_url, redis_prefix)?)
        }
        SchedulerBackend::Postgres => {
            let database_url = queue
                    .scheduler_database_url
                    .as_deref()
                    .or(storage_database_url)
                    .ok_or_else(|| {
                        anyhow!(
                            "postgres scheduler requires TARDIGRADE_SCHEDULER_DATABASE_URL or TARDIGRADE_DATABASE_URL"
                        )
                    })?;
            let namespace = queue.scheduler_namespace.as_str();
            info!(namespace = %namespace, "using postgres-backed scheduler");
            Arc::new(PostgresScheduler::open(database_url, namespace)?)
        }
    };

    Ok((scheduler, selected_backend))
}

/// Logs queue file usage hints when a queue file path is present.
fn log_queue_file_usage(queue_file: Option<&str>, selected_backend: SchedulerBackend) {
    if let Some(path) = queue_file {
        if selected_backend == SchedulerBackend::File {
            info!(queue_file = %path, "queue file configured for file-backed scheduler");
        } else {
            warn!(
                queue_file = %path,
                selected_backend = ?selected_backend,
                "TARDIGRADE_QUEUE_FILE is set but only used by file scheduler backend"
            );
        }
    };
}

/// Starts SCM polling only when enabled in runtime configuration.
fn start_scm_polling_if_enabled(state: &ApiState, scm: &ScmConfig) {
    if scm.is_polling_enabled {
        state.start_scm_polling_loop(Duration::from_secs(scm.polling_check_interval));
        info!(
            scm_polling_check_secs = scm.polling_check_interval,
            "SCM polling loop enabled"
        );
    }
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

    // Share one storage backend across request handlers through an Arc trait object.
    let storage = build_storage(runtime_mode, database_url.as_deref()).await?;

    // Same pattern for the scheduler: runtime decides concrete impl, API keeps trait abstraction.
    let (scheduler, selected_scheduler_backend) =
        build_scheduler(runtime_mode, &queue, database_url.as_deref())?;
    log_queue_file_usage(queue.queue_file.as_deref(), selected_scheduler_backend);
    let run_embedded_worker = has_embedded_worker;
    let state = ApiState::with_components_and_mode(
        service_name.clone(),
        storage,
        scheduler,
        run_embedded_worker,
    );
    start_scm_polling_if_enabled(&state, &scm);

    let router = mount_dashboard_assets(mount_webhook_adapter(build_router(state.clone()), state));

    // Bind socket first, then hand listener to Axum for graceful shutdown support.
    let bind_addr = bind_address;
    let listener = TcpListener::bind(&bind_addr).await?;
    info!(bind_addr = %bind_addr, run_embedded_worker, "server listening");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
