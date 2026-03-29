use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Lifecycle status for a build execution in the CI control-plane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobStatus {
    /// Pending means queued but not yet claimed by a worker.
    Pending,
    /// Running means a worker currently owns and executes the build.
    Running,
    /// Success/Failed/Canceled are terminal states.
    Success,
    Failed,
    Canceled,
}

/// Immutable job declaration describing where code lives and which pipeline to run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDefinition {
    pub id: Uuid,
    pub name: String,
    pub repository_url: String,
    pub pipeline_path: String,
    pub created_at: DateTime<Utc>,
}

/// Mutable runtime record tracking one execution attempt of a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRecord {
    pub id: Uuid,
    pub job_id: Uuid,
    pub status: JobStatus,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub logs: Vec<String>,
}

impl JobDefinition {
    /// Creates a new job definition with generated id and creation timestamp.
    pub fn new(name: impl Into<String>, repository_url: impl Into<String>, pipeline_path: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            repository_url: repository_url.into(),
            pipeline_path: pipeline_path.into(),
            created_at: Utc::now(),
        }
    }
}

impl BuildRecord {
    /// Creates a newly queued build record tied to a job id.
    pub fn queued(job_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            job_id,
            status: JobStatus::Pending,
            queued_at: Utc::now(),
            started_at: None,
            finished_at: None,
            logs: Vec::new(),
        }
    }

    /// Marks the build as running when claimed by a worker.
    pub fn mark_running(&mut self) -> bool {
        // State transition guard: only pending builds can be claimed.
        if self.status != JobStatus::Pending {
            return false;
        }

        self.status = JobStatus::Running;
        self.started_at = Some(Utc::now());
        true
    }

    /// Marks the build as successful after worker completion.
    pub fn mark_success(&mut self) -> bool {
        // State transition guard: success can only come from running.
        if self.status != JobStatus::Running {
            return false;
        }

        self.status = JobStatus::Success;
        self.finished_at = Some(Utc::now());
        true
    }

    /// Marks the build as failed after worker completion.
    pub fn mark_failed(&mut self) -> bool {
        // State transition guard: failure can only come from running.
        if self.status != JobStatus::Running {
            return false;
        }

        self.status = JobStatus::Failed;
        self.finished_at = Some(Utc::now());
        true
    }

    /// Cancels a non-terminal build from API/operator action.
    pub fn cancel(&mut self) -> bool {
        // Cancellation is a no-op once a build reached a terminal state.
        if matches!(
            self.status,
            JobStatus::Success | JobStatus::Failed | JobStatus::Canceled
        ) {
            return false;
        }

        self.status = JobStatus::Canceled;
        self.finished_at = Some(Utc::now());
        true
    }

    /// Requeues a running build back to pending for retry/reclaim flows.
    pub fn requeue_from_running(&mut self) -> bool {
        // Requeue resets execution timestamps so retries/stale reclaims look like fresh attempts.
        if self.status != JobStatus::Running {
            return false;
        }

        self.status = JobStatus::Pending;
        self.started_at = None;
        self.finished_at = None;
        true
    }

    /// Appends a human-readable log line to build execution history.
    pub fn append_log(&mut self, line: impl Into<String>) {
        self.logs.push(line.into());
    }
}
