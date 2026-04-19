use anyhow::Result;
use chrono::Utc;
use redis::Commands;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

use crate::Scheduler;
use crate::model::InFlightEntry;

/// Redis-backed scheduler for distributed API/worker deployments.
#[derive(Clone)]
pub struct RedisScheduler {
    connection: Arc<Mutex<redis::Connection>>,
    queue_key: Arc<String>,
    in_flight_key: Arc<String>,
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
        let encoded: Option<String> =
            connection.hget(self.in_flight_key.as_str(), build_id.to_string())?;
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

    /// Removes one canceled build from redis queue and in-flight lease hash.
    fn deschedule(&self, build_id: Uuid) -> Result<()> {
        let build_id_str = build_id.to_string();
        let mut connection = self.connection.lock().expect("redis queue poisoned");
        let _: usize = connection.hdel(self.in_flight_key.as_str(), &build_id_str)?;
        let _: usize = connection.lrem(self.queue_key.as_str(), 0, build_id_str)?;
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
