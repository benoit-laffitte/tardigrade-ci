use anyhow::Result;
use tardigrade_core::BuildRecord;
use tracing::info;

/// Worker execution adapter used by embedded-worker mode.
pub struct WorkerExecutor;

impl WorkerExecutor {
    /// Executes one build and returns updated record.
    pub async fn run(mut build: BuildRecord) -> Result<BuildRecord> {
        // Executor currently simulates a build run and marks success.
        info!(build_id = %build.id, "starting build execution");
        build.append_log("Executor started job");

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        if build.mark_success() {
            build.append_log("Executor finished job successfully");
        }

        Ok(build)
    }
}
