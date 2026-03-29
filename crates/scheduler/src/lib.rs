use anyhow::Result;
use chrono::{DateTime, Utc};
use redis::Commands;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

/// Queue contract abstracting claim/ack/requeue semantics across backends.
pub trait Scheduler: Send + Sync {
    /// Enqueue appends a new build to be claimed by workers.
    fn enqueue(&self, build_id: Uuid) -> Result<()>;
    /// claim_next transfers ownership to a worker and moves it to in-flight.
    fn claim_next(&self, worker_id: &str) -> Option<Uuid>;
    /// reclaim_stale returns builds whose lease exceeded max_age and requeues them.
    fn reclaim_stale(&self, max_age: Duration) -> Result<Vec<Uuid>>;
    /// in_flight_owner is used by completion API to enforce worker ownership.
    fn in_flight_owner(&self, build_id: Uuid) -> Result<Option<String>>;
    fn ack(&self, build_id: Uuid) -> Result<()>;
    fn requeue(&self, build_id: Uuid) -> Result<()>;
    /// worker_loads powers dashboard visibility and readiness checks.
    fn worker_loads(&self) -> HashMap<String, usize>;
}

/// Volatile in-memory scheduler implementation.
#[derive(Clone, Default)]
pub struct InMemoryScheduler {
    state: Arc<Mutex<QueueState>>,
}

/// Serializable queue state used by in-memory and file-backed schedulers.
#[derive(Debug, Default, Serialize, Deserialize)]
struct QueueState {
    queue: VecDeque<Uuid>,
    in_flight: HashMap<Uuid, InFlightEntry>,
}

/// In-flight lease metadata for ownership and stale-claim detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InFlightEntry {
    worker_id: String,
    claimed_at: DateTime<Utc>,
}

/// Durable scheduler persisting queue/in-flight state to a json file.
#[derive(Clone)]
pub struct FileBackedScheduler {
    state: Arc<Mutex<QueueState>>,
    state_file: Arc<PathBuf>,
}

/// Redis-backed scheduler for distributed API/worker deployments.
#[derive(Clone)]
pub struct RedisScheduler {
    connection: Arc<Mutex<redis::Connection>>,
    queue_key: Arc<String>,
    in_flight_key: Arc<String>,
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

impl RedisScheduler {
    /// Opens redis connection and derives key names from prefix.
    pub fn open(redis_url: &str, key_prefix: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let connection = client.get_connection()?;

        let queue_key = format!("{key_prefix}:queue");
        let in_flight_key = format!("{key_prefix}:in_flight");

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            queue_key: Arc::new(queue_key),
            in_flight_key: Arc::new(in_flight_key),
        })
    }
}

impl Scheduler for InMemoryScheduler {
    /// Pushes build id at tail of queue.
    fn enqueue(&self, build_id: Uuid) -> Result<()> {
        let mut state = self.state.lock().expect("scheduler queue poisoned");
        state.queue.push_back(build_id);
        Ok(())
    }

    /// Pops queue head and records worker lease in in-flight map.
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
        Some(build_id)
    }

    /// Reclaims in-flight entries older than max_age and requeues them.
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
            // Reclaimed builds are pushed to the front so they are retried quickly.
            state.in_flight.remove(&build_id);
            state.queue.push_front(build_id);
            reclaimed.push(build_id);
        }

        Ok(reclaimed)
    }

    /// Returns current in-flight owner for a build.
    fn in_flight_owner(&self, build_id: Uuid) -> Result<Option<String>> {
        let state = self.state.lock().expect("scheduler queue poisoned");
        Ok(state.in_flight.get(&build_id).map(|e| e.worker_id.clone()))
    }

    /// Acknowledges completion and removes build from in-flight set.
    fn ack(&self, build_id: Uuid) -> Result<()> {
        let mut state = self.state.lock().expect("scheduler queue poisoned");
        state.in_flight.remove(&build_id);
        Ok(())
    }

    /// Requeues build at front and clears any stale in-flight ownership.
    fn requeue(&self, build_id: Uuid) -> Result<()> {
        let mut state = self.state.lock().expect("scheduler queue poisoned");
        state.in_flight.remove(&build_id);
        state.queue.push_front(build_id);
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

impl Scheduler for RedisScheduler {
    /// Pushes build id to redis list tail.
    fn enqueue(&self, build_id: Uuid) -> Result<()> {
        let mut connection = self.connection.lock().expect("redis queue poisoned");
        let _: usize = connection.rpush(self.queue_key.as_str(), build_id.to_string())?;
        Ok(())
    }

    /// Atomically-like claim via list pop + hash lease write (best-effort rollback on failure).
    fn claim_next(&self, worker_id: &str) -> Option<Uuid> {
        let mut connection = self.connection.lock().expect("redis queue poisoned");
        let build_id_str: Option<String> = connection.lpop(self.queue_key.as_str(), None).ok()?;
        let build_id_str = build_id_str?;

        let entry = InFlightEntry {
            worker_id: worker_id.to_string(),
            claimed_at: Utc::now(),
        };
        let encoded = serde_json::to_string(&entry).ok()?;

        if connection
            .hset::<_, _, _, usize>(self.in_flight_key.as_str(), &build_id_str, encoded)
            .is_err()
        {
            // Best effort rollback to the front of queue if inflight write fails.
            let _ = connection.lpush::<_, _, usize>(self.queue_key.as_str(), &build_id_str);
            return None;
        }

        Uuid::parse_str(&build_id_str).ok()
    }

    /// Reclaims stale redis hash leases and pushes reclaimed builds to queue front.
    fn reclaim_stale(&self, max_age: Duration) -> Result<Vec<Uuid>> {
        let mut connection = self.connection.lock().expect("redis queue poisoned");
        let entries: HashMap<String, String> = connection.hgetall(self.in_flight_key.as_str())?;
        let now = Utc::now();
        let mut reclaimed = Vec::new();

        for (build_id_str, encoded) in entries {
            let Ok(entry) = serde_json::from_str::<InFlightEntry>(&encoded) else {
                continue;
            };
            let age = now
                .signed_duration_since(entry.claimed_at)
                .to_std()
                .unwrap_or(Duration::from_secs(0));

            if age >= max_age {
                // Remove stale lease then requeue build for another worker.
                let _: usize = connection.hdel(self.in_flight_key.as_str(), &build_id_str)?;
                let _: usize = connection.lpush(self.queue_key.as_str(), &build_id_str)?;
                if let Ok(build_id) = Uuid::parse_str(&build_id_str) {
                    reclaimed.push(build_id);
                }
            }
        }

        Ok(reclaimed)
    }

    /// Reads lease owner from redis in-flight hash.
    fn in_flight_owner(&self, build_id: Uuid) -> Result<Option<String>> {
        let mut connection = self.connection.lock().expect("redis queue poisoned");
        let encoded: Option<String> = connection.hget(self.in_flight_key.as_str(), build_id.to_string())?;
        let owner = encoded
            .and_then(|payload| serde_json::from_str::<InFlightEntry>(&payload).ok())
            .map(|entry| entry.worker_id);
        Ok(owner)
    }

    /// Removes lease owner from redis in-flight hash.
    fn ack(&self, build_id: Uuid) -> Result<()> {
        let mut connection = self.connection.lock().expect("redis queue poisoned");
        let _: usize = connection.hdel(self.in_flight_key.as_str(), build_id.to_string())?;
        Ok(())
    }

    /// Removes lease and requeues build at redis list head.
    fn requeue(&self, build_id: Uuid) -> Result<()> {
        let build_id_str = build_id.to_string();
        let mut connection = self.connection.lock().expect("redis queue poisoned");
        let _: usize = connection.hdel(self.in_flight_key.as_str(), &build_id_str)?;
        let _: usize = connection.lpush(self.queue_key.as_str(), build_id_str)?;
        Ok(())
    }

    /// Computes per-worker active loads from redis in-flight hash values.
    fn worker_loads(&self) -> HashMap<String, usize> {
        let mut connection = self.connection.lock().expect("redis queue poisoned");
        let encoded_entries: Vec<String> = connection
            .hvals(self.in_flight_key.as_str())
            .unwrap_or_default();

        let mut loads = HashMap::new();
        for encoded in encoded_entries {
            if let Ok(entry) = serde_json::from_str::<InFlightEntry>(&encoded) {
                *loads.entry(entry.worker_id).or_insert(0) += 1;
            }
        }

        loads
    }
}

#[cfg(test)]
mod tests;
