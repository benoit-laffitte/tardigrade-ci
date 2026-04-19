use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

use crate::Scheduler;
use crate::model::{InFlightEntry, QueueState};

/// Volatile in-memory scheduler implementation.
#[derive(Clone, Default)]
pub struct InMemoryScheduler {
    state: Arc<Mutex<QueueState>>,
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

    /// Removes one build from queue and in-flight state when cancellation is requested.
    fn deschedule(&self, build_id: Uuid) -> Result<()> {
        let mut state = self.state.lock().expect("scheduler queue poisoned");
        state.in_flight.remove(&build_id);
        state.queue.retain(|queued| *queued != build_id);
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
