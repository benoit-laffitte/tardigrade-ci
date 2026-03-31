use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

use crate::model::{InFlightEntry, QueueState};
use crate::Scheduler;

/// Durable scheduler persisting queue/in-flight state to a json file.
#[derive(Clone)]
pub struct FileBackedScheduler {
    state: Arc<Mutex<QueueState>>,
    state_file: Arc<PathBuf>,
}

impl FileBackedScheduler {
    /// Opens scheduler from disk and initializes state file when missing.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let state = if path.exists() {
            let raw = fs::read_to_string(&path)?;
            if raw.trim().is_empty() {
                QueueState::default()
            } else {
                serde_json::from_str(&raw)?
            }
        } else {
            QueueState::default()
        };

        let scheduler = Self {
            state: Arc::new(Mutex::new(state)),
            state_file: Arc::new(path),
        };

        // Ensure state file exists with valid JSON from first initialization.
        {
            let snapshot = scheduler.state.lock().expect("scheduler queue poisoned");
            scheduler.persist_state(&snapshot)?;
        }

        Ok(scheduler)
    }

    /// Persists scheduler state atomically via temporary file swap.
    fn persist_state(&self, state: &QueueState) -> Result<()> {
        // Atomic write via temp file prevents truncated/corrupted queue snapshots.
        let tmp_path = self.state_file.with_extension("tmp");
        let payload = serde_json::to_vec_pretty(state)?;
        fs::write(&tmp_path, payload)?;
        fs::rename(tmp_path, self.state_file.as_ref())?;
        Ok(())
    }
}

impl Scheduler for FileBackedScheduler {
    /// Pushes build id at tail and persists state on disk.
    fn enqueue(&self, build_id: Uuid) -> Result<()> {
        let mut state = self.state.lock().expect("scheduler queue poisoned");
        state.queue.push_back(build_id);
        self.persist_state(&state)?;
        Ok(())
    }

    /// Claims queue head for worker and persists lease ownership.
    fn claim_next(&self, worker_id: &str) -> Option<Uuid> {
        let mut state = self.state.lock().expect("scheduler queue poisoned");
        let build_id = state.queue.pop_front()?;
        state.in_flight.insert(
            build_id,
            InFlightEntry {
                worker_id: worker_id.to_string(),
                claimed_at: Utc::now(),
            },
        );
        if self.persist_state(&state).is_err() {
            // Roll back in-memory mutation when persistence fails.
            state.in_flight.remove(&build_id);
            state.queue.push_front(build_id);
            return None;
        }

        Some(build_id)
    }

    /// Reclaims stale in-flight entries and persists updated state.
    fn reclaim_stale(&self, max_age: Duration) -> Result<Vec<Uuid>> {
        let mut state = self.state.lock().expect("scheduler queue poisoned");
        let now = Utc::now();
        let mut reclaimed = Vec::new();

        let stale_ids = state
            .in_flight
            .iter()
            .filter_map(|(build_id, entry)| {
                let age = now
                    .signed_duration_since(entry.claimed_at)
                    .to_std()
                    .unwrap_or(Duration::from_secs(0));
                if age >= max_age {
                    Some(*build_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for build_id in stale_ids {
            state.in_flight.remove(&build_id);
            state.queue.push_front(build_id);
            reclaimed.push(build_id);
        }

        self.persist_state(&state)?;
        Ok(reclaimed)
    }

    /// Returns current in-flight owner for a build.
    fn in_flight_owner(&self, build_id: Uuid) -> Result<Option<String>> {
        let state = self.state.lock().expect("scheduler queue poisoned");
        Ok(state.in_flight.get(&build_id).map(|e| e.worker_id.clone()))
    }

    /// Acknowledges completion and persists removal from in-flight map.
    fn ack(&self, build_id: Uuid) -> Result<()> {
        let mut state = self.state.lock().expect("scheduler queue poisoned");
        state.in_flight.remove(&build_id);
        self.persist_state(&state)?;
        Ok(())
    }

    /// Requeues build and persists queue/in-flight update.
    fn requeue(&self, build_id: Uuid) -> Result<()> {
        let mut state = self.state.lock().expect("scheduler queue poisoned");
        state.in_flight.remove(&build_id);
        state.queue.push_front(build_id);
        self.persist_state(&state)?;
        Ok(())
    }

    /// Aggregates active build count by worker id.
    fn worker_loads(&self) -> HashMap<String, usize> {
        let state = self.state.lock().expect("scheduler queue poisoned");
        let mut loads = HashMap::new();
        for entry in state.in_flight.values() {
            *loads.entry(entry.worker_id.clone()).or_insert(0) += 1;
        }

        loads
    }
}
