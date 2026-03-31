use anyhow::Result;
use std::collections::HashMap;
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

    /// ack confirms completion and clears in-flight ownership.
    fn ack(&self, build_id: Uuid) -> Result<()>;

    /// requeue pushes one build back to queue for retry.
    fn requeue(&self, build_id: Uuid) -> Result<()>;

    /// worker_loads powers dashboard visibility and readiness checks.
    fn worker_loads(&self) -> HashMap<String, usize>;
}
