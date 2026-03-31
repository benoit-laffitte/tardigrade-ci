use chrono::{DateTime, Utc};
use std::time::Duration;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use tardigrade_scheduler::Scheduler;
use tardigrade_storage::Storage;
use tokio::sync::broadcast;
use uuid::Uuid;

use super::RuntimeMetrics;
use crate::LiveEvent;

/// Service owns all domain orchestration (storage, scheduler, metrics, events).
#[derive(Clone)]
pub(crate) struct CiService {
    pub(crate) storage: Arc<dyn Storage + Send + Sync>,
    pub(crate) scheduler: Arc<dyn Scheduler + Send + Sync>,
    /// last_seen map allows the dashboard to expose active/idle workers.
    pub(crate) worker_registry: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    pub(crate) worker_lease_timeout: Duration,
    pub(crate) max_retries: u32,
    pub(crate) retry_backoff_ms: u64,
    /// retry_state tracks attempt count per build until terminal state.
    pub(crate) retry_state: Arc<Mutex<HashMap<Uuid, u32>>>,
    pub(crate) metrics: Arc<Mutex<RuntimeMetrics>>,
    /// dead_letter_builds provides a focused operational view over failed terminal retries.
    pub(crate) dead_letter_builds: Arc<Mutex<HashSet<Uuid>>>,
    /// seen_webhook_events stores recent dedup keys to enforce idempotent ingestion.
    pub(crate) seen_webhook_events: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    pub(crate) webhook_dedup_ttl: Duration,
    /// Internal broadcast bus feeding the SSE /events endpoint.
    pub(crate) event_tx: broadcast::Sender<LiveEvent>,
}
