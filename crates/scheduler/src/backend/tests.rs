use super::{FileBackedScheduler, InMemoryScheduler, PostgresScheduler};
use crate::Scheduler;
use std::time::Duration;
use uuid::Uuid;

/// Verifies queue state survives process restart with file-backed scheduler.
#[test]
fn file_backed_scheduler_persists_queue_state() {
    let state_file =
        std::env::temp_dir().join(format!("tardigrade-scheduler-{}.json", Uuid::new_v4()));

    let first_build = Uuid::new_v4();
    let second_build = Uuid::new_v4();

    {
        let scheduler = FileBackedScheduler::open(&state_file).expect("open scheduler");
        scheduler.enqueue(first_build).expect("enqueue first build");
        scheduler
            .enqueue(second_build)
            .expect("enqueue second build");

        let claimed = scheduler
            .claim_next("worker-a")
            .expect("claim should return first build");
        assert_eq!(claimed, first_build);
    }

    {
        let scheduler = FileBackedScheduler::open(&state_file).expect("reopen scheduler");
        let claimed = scheduler
            .claim_next("worker-b")
            .expect("claim should return queued build");
        assert_eq!(claimed, second_build);
    }

    let _ = std::fs::remove_file(state_file);
}

/// Verifies stale lease reclaim makes build claimable by another worker.
#[test]
fn in_memory_scheduler_can_reclaim_stale_builds() {
    let scheduler = InMemoryScheduler::default();
    let build_id = Uuid::new_v4();

    scheduler.enqueue(build_id).expect("enqueue build");
    let claimed = scheduler.claim_next("worker-a").expect("claim build");
    assert_eq!(claimed, build_id);

    let reclaimed = scheduler
        .reclaim_stale(Duration::from_secs(0))
        .expect("reclaim stale build");
    assert_eq!(reclaimed, vec![build_id]);

    let claimed_again = scheduler
        .claim_next("worker-b")
        .expect("claim reclaimed build");
    assert_eq!(claimed_again, build_id);
}

/// Verifies ownership tracking, load counters, and ack semantics in memory scheduler.
#[test]
fn in_memory_scheduler_tracks_owner_load_and_ack() {
    let scheduler = InMemoryScheduler::default();
    let build_a = Uuid::new_v4();
    let build_b = Uuid::new_v4();

    scheduler.enqueue(build_a).expect("enqueue build a");
    scheduler.enqueue(build_b).expect("enqueue build b");

    let claimed_a = scheduler
        .claim_next("worker-a")
        .expect("worker-a should claim build a");
    let claimed_b = scheduler
        .claim_next("worker-b")
        .expect("worker-b should claim build b");

    assert_eq!(claimed_a, build_a);
    assert_eq!(claimed_b, build_b);
    assert_eq!(
        scheduler
            .in_flight_owner(build_a)
            .expect("in_flight_owner should succeed"),
        Some("worker-a".to_string())
    );
    assert_eq!(
        scheduler
            .in_flight_owner(build_b)
            .expect("in_flight_owner should succeed"),
        Some("worker-b".to_string())
    );

    let loads = scheduler.worker_loads();
    assert_eq!(loads.get("worker-a"), Some(&1));
    assert_eq!(loads.get("worker-b"), Some(&1));

    scheduler.ack(build_a).expect("ack should succeed");
    assert_eq!(
        scheduler
            .in_flight_owner(build_a)
            .expect("in_flight_owner should succeed"),
        None
    );

    let loads_after_ack = scheduler.worker_loads();
    assert_eq!(loads_after_ack.get("worker-a"), None);
    assert_eq!(loads_after_ack.get("worker-b"), Some(&1));
}

/// Verifies requeue clears old ownership and makes build claimable again.
#[test]
fn in_memory_scheduler_requeue_clears_ownership_and_reclaims_build() {
    let scheduler = InMemoryScheduler::default();
    let build_id = Uuid::new_v4();

    scheduler.enqueue(build_id).expect("enqueue should succeed");
    let first_claim = scheduler
        .claim_next("worker-a")
        .expect("initial claim should succeed");
    assert_eq!(first_claim, build_id);

    scheduler.requeue(build_id).expect("requeue should succeed");
    assert_eq!(
        scheduler
            .in_flight_owner(build_id)
            .expect("in_flight_owner should succeed"),
        None
    );

    let second_claim = scheduler
        .claim_next("worker-b")
        .expect("reclaimed build should be claimable");
    assert_eq!(second_claim, build_id);
    assert_eq!(
        scheduler
            .in_flight_owner(build_id)
            .expect("in_flight_owner should succeed"),
        Some("worker-b".to_string())
    );
}

/// Verifies file-backed scheduler persists requeue and ack transitions across reopen.
#[test]
fn file_backed_scheduler_persists_requeue_and_ack_transitions() {
    let state_file = std::env::temp_dir().join(format!(
        "tardigrade-scheduler-requeue-{}.json",
        Uuid::new_v4()
    ));

    let build_id = Uuid::new_v4();
    {
        let scheduler = FileBackedScheduler::open(&state_file).expect("open scheduler");
        scheduler.enqueue(build_id).expect("enqueue build");
        let claimed = scheduler
            .claim_next("worker-a")
            .expect("claim should return build");
        assert_eq!(claimed, build_id);
        scheduler.requeue(build_id).expect("requeue should succeed");
    }

    {
        let scheduler = FileBackedScheduler::open(&state_file).expect("reopen scheduler");
        let claimed = scheduler
            .claim_next("worker-b")
            .expect("requeued build should be claimable");
        assert_eq!(claimed, build_id);
        scheduler.ack(build_id).expect("ack should succeed");
        assert_eq!(
            scheduler
                .in_flight_owner(build_id)
                .expect("in_flight_owner should succeed"),
            None
        );
    }

    let _ = std::fs::remove_file(state_file);
}

/// Verifies claiming from an empty file-backed scheduler returns none.
#[test]
fn file_backed_scheduler_claim_next_returns_none_when_empty() {
    let state_file = std::env::temp_dir().join(format!(
        "tardigrade-scheduler-empty-{}.json",
        Uuid::new_v4()
    ));
    let scheduler = FileBackedScheduler::open(&state_file).expect("open scheduler");

    assert_eq!(scheduler.claim_next("worker-a"), None);

    let _ = std::fs::remove_file(state_file);
}

/// Verifies reclaim with a large timeout keeps active in-flight claims untouched.
#[test]
fn in_memory_scheduler_reclaim_stale_noop_for_recent_claims() {
    let scheduler = InMemoryScheduler::default();
    let build_id = Uuid::new_v4();

    scheduler.enqueue(build_id).expect("enqueue build");
    let claimed = scheduler
        .claim_next("worker-a")
        .expect("claim should succeed");
    assert_eq!(claimed, build_id);

    let reclaimed = scheduler
        .reclaim_stale(Duration::from_secs(3600))
        .expect("reclaim should succeed");
    assert!(reclaimed.is_empty());
    assert_eq!(
        scheduler
            .in_flight_owner(build_id)
            .expect("in_flight_owner should succeed"),
        Some("worker-a".to_string())
    );
}

/// Verifies file-backed scheduler exposes worker loads while builds are in-flight.
#[test]
fn file_backed_scheduler_reports_worker_loads() {
    let state_file = std::env::temp_dir().join(format!(
        "tardigrade-scheduler-loads-{}.json",
        Uuid::new_v4()
    ));
    let build_a = Uuid::new_v4();
    let build_b = Uuid::new_v4();

    let scheduler = FileBackedScheduler::open(&state_file).expect("open scheduler");
    scheduler.enqueue(build_a).expect("enqueue build a");
    scheduler.enqueue(build_b).expect("enqueue build b");

    let claimed_a = scheduler.claim_next("worker-a").expect("claim build a");
    let claimed_b = scheduler.claim_next("worker-a").expect("claim build b");
    assert_eq!(claimed_a, build_a);
    assert_eq!(claimed_b, build_b);

    let loads = scheduler.worker_loads();
    assert_eq!(loads.get("worker-a"), Some(&2));

    scheduler.ack(build_a).expect("ack build a");
    scheduler.ack(build_b).expect("ack build b");
    assert!(scheduler.worker_loads().is_empty());

    let _ = std::fs::remove_file(state_file);
}

/// Verifies postgres-backed scheduler claim/requeue/ack semantics when test database is configured.
#[test]
fn postgres_scheduler_supports_claim_requeue_and_ack() {
    let database_url = match std::env::var("TARDIGRADE_TEST_DATABASE_URL") {
        Ok(value) => value,
        Err(_) => return,
    };
    let namespace = format!("scheduler-test-{}", Uuid::new_v4());
    let scheduler = PostgresScheduler::open(&database_url, &namespace).expect("open scheduler");
    let build_id = Uuid::new_v4();

    scheduler.enqueue(build_id).expect("enqueue should succeed");
    let claimed = scheduler
        .claim_next("worker-a")
        .expect("claim should return enqueued build");
    assert_eq!(claimed, build_id);
    assert_eq!(
        scheduler
            .in_flight_owner(build_id)
            .expect("in_flight_owner should succeed"),
        Some("worker-a".to_string())
    );

    scheduler.requeue(build_id).expect("requeue should succeed");
    let reclaimed = scheduler
        .claim_next("worker-b")
        .expect("requeued build should be claimable");
    assert_eq!(reclaimed, build_id);

    scheduler.ack(build_id).expect("ack should succeed");
    assert_eq!(
        scheduler
            .in_flight_owner(build_id)
            .expect("in_flight_owner should succeed"),
        None
    );
}
