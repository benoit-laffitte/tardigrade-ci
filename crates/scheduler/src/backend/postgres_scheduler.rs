use anyhow::Result;
use chrono::Utc;
use postgres::{Client, NoTls};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

use crate::Scheduler;

/// PostgreSQL-backed scheduler for durable multi-instance queue coordination.
#[derive(Clone)]
pub struct PostgresScheduler {
    connection: Arc<Mutex<Client>>,
    namespace: Arc<String>,
}

impl PostgresScheduler {
    /// Opens postgres connection and ensures queue/in-flight tables are present.
    pub fn open(database_url: &str, namespace: &str) -> Result<Self> {
        let mut connection = Client::connect(database_url, NoTls)?;

        connection.batch_execute(
            r#"
            CREATE TABLE IF NOT EXISTS scheduler_queue_entries (
                namespace TEXT NOT NULL,
                sequence BIGSERIAL NOT NULL,
                build_id UUID NOT NULL,
                priority SMALLINT NOT NULL DEFAULT 0,
                enqueued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                PRIMARY KEY (namespace, sequence),
                UNIQUE (namespace, build_id)
            );

            CREATE INDEX IF NOT EXISTS scheduler_queue_claim_idx
            ON scheduler_queue_entries (namespace, priority DESC, sequence ASC);

            CREATE TABLE IF NOT EXISTS scheduler_in_flight_entries (
                namespace TEXT NOT NULL,
                build_id UUID NOT NULL,
                worker_id TEXT NOT NULL,
                claimed_at TIMESTAMPTZ NOT NULL,
                PRIMARY KEY (namespace, build_id)
            );

            CREATE INDEX IF NOT EXISTS scheduler_in_flight_stale_idx
            ON scheduler_in_flight_entries (namespace, claimed_at ASC);
            "#,
        )?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            namespace: Arc::new(namespace.to_string()),
        })
    }
}

impl Scheduler for PostgresScheduler {
    /// Pushes build id to queue tail with normal priority.
    fn enqueue(&self, build_id: Uuid) -> Result<()> {
        let mut connection = self.connection.lock().expect("postgres queue poisoned");
        connection.execute(
            r#"
            INSERT INTO scheduler_queue_entries (namespace, build_id, priority)
            VALUES ($1, $2, 0)
            ON CONFLICT (namespace, build_id) DO NOTHING
            "#,
            &[&self.namespace.as_str(), &build_id],
        )?;
        Ok(())
    }

    /// Claims next build by priority and records worker lease in one transaction.
    fn claim_next(&self, worker_id: &str) -> Option<Uuid> {
        let mut connection = self.connection.lock().expect("postgres queue poisoned");
        let mut tx = connection.transaction().ok()?;

        let row = tx
            .query_opt(
                r#"
                SELECT sequence, build_id
                FROM scheduler_queue_entries
                WHERE namespace = $1
                ORDER BY priority DESC, sequence ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
                "#,
                &[&self.namespace.as_str()],
            )
            .ok()?;

        let row = row?;
        let sequence: i64 = row.get("sequence");
        let build_id: Uuid = row.get("build_id");

        tx.execute(
            r#"
            DELETE FROM scheduler_queue_entries
            WHERE namespace = $1 AND sequence = $2
            "#,
            &[&self.namespace.as_str(), &sequence],
        )
        .ok()?;

        tx.execute(
            r#"
            INSERT INTO scheduler_in_flight_entries (namespace, build_id, worker_id, claimed_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (namespace, build_id) DO UPDATE
            SET worker_id = EXCLUDED.worker_id,
                claimed_at = EXCLUDED.claimed_at
            "#,
            &[&self.namespace.as_str(), &build_id, &worker_id],
        )
        .ok()?;

        tx.commit().ok()?;
        Some(build_id)
    }

    /// Reclaims stale leases and requeues reclaimed builds with retry priority.
    fn reclaim_stale(&self, max_age: Duration) -> Result<Vec<Uuid>> {
        let max_age =
            chrono::Duration::from_std(max_age).unwrap_or_else(|_| chrono::Duration::seconds(0));
        let stale_before = Utc::now() - max_age;

        let mut connection = self.connection.lock().expect("postgres queue poisoned");
        let mut tx = connection.transaction()?;

        let stale_rows = tx.query(
            r#"
            SELECT build_id
            FROM scheduler_in_flight_entries
            WHERE namespace = $1 AND claimed_at <= $2
            FOR UPDATE
            "#,
            &[&self.namespace.as_str(), &stale_before],
        )?;

        let mut reclaimed = Vec::new();
        for row in stale_rows {
            let build_id: Uuid = row.get("build_id");
            tx.execute(
                r#"
                DELETE FROM scheduler_in_flight_entries
                WHERE namespace = $1 AND build_id = $2
                "#,
                &[&self.namespace.as_str(), &build_id],
            )?;

            tx.execute(
                r#"
                INSERT INTO scheduler_queue_entries (namespace, build_id, priority, enqueued_at)
                VALUES ($1, $2, 1, NOW())
                ON CONFLICT (namespace, build_id) DO UPDATE
                SET priority = 1,
                    enqueued_at = NOW()
                "#,
                &[&self.namespace.as_str(), &build_id],
            )?;

            reclaimed.push(build_id);
        }

        tx.commit()?;
        Ok(reclaimed)
    }

    /// Returns current in-flight owner for a build.
    fn in_flight_owner(&self, build_id: Uuid) -> Result<Option<String>> {
        let mut connection = self.connection.lock().expect("postgres queue poisoned");
        let owner = connection
            .query_opt(
                r#"
                SELECT worker_id
                FROM scheduler_in_flight_entries
                WHERE namespace = $1 AND build_id = $2
                "#,
                &[&self.namespace.as_str(), &build_id],
            )?
            .map(|row| row.get("worker_id"));

        Ok(owner)
    }

    /// Acknowledges completion by removing build lease ownership.
    fn ack(&self, build_id: Uuid) -> Result<()> {
        let mut connection = self.connection.lock().expect("postgres queue poisoned");
        connection.execute(
            r#"
            DELETE FROM scheduler_in_flight_entries
            WHERE namespace = $1 AND build_id = $2
            "#,
            &[&self.namespace.as_str(), &build_id],
        )?;
        Ok(())
    }

    /// Clears lease ownership and requeues build with retry priority.
    fn requeue(&self, build_id: Uuid) -> Result<()> {
        let mut connection = self.connection.lock().expect("postgres queue poisoned");
        let mut tx = connection.transaction()?;

        tx.execute(
            r#"
            DELETE FROM scheduler_in_flight_entries
            WHERE namespace = $1 AND build_id = $2
            "#,
            &[&self.namespace.as_str(), &build_id],
        )?;

        tx.execute(
            r#"
            INSERT INTO scheduler_queue_entries (namespace, build_id, priority, enqueued_at)
            VALUES ($1, $2, 1, NOW())
            ON CONFLICT (namespace, build_id) DO UPDATE
            SET priority = 1,
                enqueued_at = NOW()
            "#,
            &[&self.namespace.as_str(), &build_id],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Computes active in-flight load grouped by worker id.
    fn worker_loads(&self) -> HashMap<String, usize> {
        let mut connection = self.connection.lock().expect("postgres queue poisoned");
        let rows = connection
            .query(
                r#"
                SELECT worker_id, COUNT(*)::BIGINT AS worker_count
                FROM scheduler_in_flight_entries
                WHERE namespace = $1
                GROUP BY worker_id
                "#,
                &[&self.namespace.as_str()],
            )
            .unwrap_or_default();

        let mut loads = HashMap::new();
        for row in rows {
            let worker_id: String = row.get("worker_id");
            let count: i64 = row.get("worker_count");
            if count > 0 {
                loads.insert(worker_id, count as usize);
            }
        }

        loads
    }
}
