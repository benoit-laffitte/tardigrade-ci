use anyhow::{Context, Result, anyhow};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tardigrade_api::{ApiState, build_router};
use tardigrade_scheduler::{
    adapters::{FileBackedScheduler, InMemoryScheduler, PostgresScheduler, RedisScheduler},
    ports::Scheduler,
};
use tardigrade_storage::{
    adapters::{InMemoryStorage, PostgresStorage},
    ports::Storage,
};
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod config;
mod dashboard;
mod runtime;
mod webhook_adapter;

use config::{RuntimeMode, ServerConfigFile};
use dashboard::{mount_dashboard_assets, resolve_web_root};
use runtime::shutdown_signal;
use webhook_adapter::mount_webhook_adapter;

/// Holds SCM polling-related configuration loaded from TOML.
struct ScmConfig {
    is_polling_enabled: bool,    // true or false
    polling_check_interval: u64, // in seconds
}

/// Holds queue and scheduler-related configuration loaded from TOML.
struct QueueConfig {
    redis_url: Option<String>,
    redis_prefix: String,
    queue_file: Option<String>,
    scheduler_backend: Option<String>,
    scheduler_database_url: Option<String>,
    scheduler_namespace: String,
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
    runtime_mode: RuntimeMode,
    service_name: String,
    bind_address: String,
    database_url: Option<String>,
    service_settings: tardigrade_api::ServiceSettings,
    web_root: std::path::PathBuf,
    scm: ScmConfig,
    queue: QueueConfig,
}

impl AppConfig {
    /// Loads the full server configuration from one TOML file.
    fn load() -> Result<Self> {
        let config_file = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "config/tardigrade-ci.toml".to_string());
        let raw = fs::read_to_string(&config_file)
            .with_context(|| format!("read config file at {config_file}"))?;
        let parsed: ServerConfigFile =
            toml::from_str(&raw).with_context(|| format!("parse TOML from {config_file}"))?;

        let runtime_mode = parsed.runtime.mode;
        let service_name = parsed.server.service_name;
        let bind_address = parsed.server.bind;
        let database_url = parsed.storage.database_url;
        let service_settings = tardigrade_api::ServiceSettings {
            worker_lease_timeout_secs: parsed.service.worker_lease_timeout_secs,
            max_retries: parsed.service.max_retries,
            retry_backoff_ms: parsed.service.retry_backoff_ms,
            webhook_dedup_ttl_secs: parsed.service.webhook_dedup_ttl_secs,
        };
        let web_root = resolve_web_root(parsed.dashboard.web_root.as_deref());

        let scm = ScmConfig {
            is_polling_enabled: parsed.scm.polling_enabled,
            polling_check_interval: parsed.scm.polling_check_secs,
        };

        let queue = QueueConfig {
            redis_url: parsed.queue.redis_url,
            redis_prefix: parsed.queue.redis_prefix,
            queue_file: parsed.queue.file_path,
            scheduler_backend: parsed.queue.backend,
            scheduler_database_url: parsed.queue.scheduler_database_url,
            scheduler_namespace: parsed.queue.scheduler_namespace,
        };

        Ok(Self {
            config_file,
            runtime_mode,
            service_name,
            bind_address,
            database_url,
            service_settings,
            web_root,
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
                .ok_or_else(|| anyhow!("prod mode requires storage.database_url in TOML"))?;
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
) -> Result<(Arc<dyn Scheduler + Send + Sync>, SchedulerBackend)> {
    let configured_backend = queue
        .scheduler_backend
        .as_deref()
        .map(|raw| {
            SchedulerBackend::parse(raw).ok_or_else(|| {
                anyhow!(
                    "invalid queue.backend value: {raw} (expected one of: in-memory, file, redis, postgres)"
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

    let scheduler: Arc<dyn Scheduler + Send + Sync> = match selected_backend {
        SchedulerBackend::InMemory => {
            info!("using in-memory scheduler");
            Arc::new(InMemoryScheduler::default())
        }
        SchedulerBackend::File => {
            let queue_file = queue
                .queue_file
                .as_deref()
                .ok_or_else(|| anyhow!("file scheduler requires queue.file_path"))?;
            info!(queue_file = %queue_file, "using file-backed scheduler");
            Arc::new(FileBackedScheduler::open(queue_file)?)
        }
        SchedulerBackend::Redis => {
            let redis_url = queue
                .redis_url
                .as_deref()
                .ok_or_else(|| anyhow!("redis scheduler requires queue.redis_url"))?;
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
                            "postgres scheduler requires queue.scheduler_database_url or storage.database_url"
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
                "queue.file_path is configured but only used by file scheduler backend"
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
    // Initialize structured logs with a deterministic default level.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    // Read runtime configuration from one TOML file.
    let app_config = AppConfig::load()?;
    let AppConfig {
        config_file,
        runtime_mode,
        service_name,
        bind_address,
        database_url,
        service_settings,
        web_root,
        scm,
        queue,
    } = app_config;

    info!(config_file = %config_file, runtime_mode = ?runtime_mode, "runtime mode loaded");
    info!(web_root = %web_root.display(), "dashboard web asset root resolved");

    // Share one storage backend across request handlers through an Arc trait object.
    let storage = build_storage(runtime_mode, database_url.as_deref()).await?;

    // Same pattern for the scheduler: runtime decides concrete impl, API keeps trait abstraction.
    let (scheduler, selected_scheduler_backend) =
        build_scheduler(runtime_mode, &queue, database_url.as_deref())?;
    log_queue_file_usage(queue.queue_file.as_deref(), selected_scheduler_backend);
    let state = ApiState::with_components_and_settings(
        service_name.clone(),
        storage,
        scheduler,
        service_settings,
    );
    start_scm_polling_if_enabled(&state, &scm);

    let router = mount_dashboard_assets(
        mount_webhook_adapter(build_router(state.clone()), state),
        web_root,
    );

    // Bind socket first, then hand listener to Axum for graceful shutdown support.
    let bind_addr = bind_address;
    let listener = TcpListener::bind(&bind_addr).await?;
    info!(bind_addr = %bind_addr, "server listening");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
