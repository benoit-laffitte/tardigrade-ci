use serde::{Deserialize, Serialize};

use super::WorkerInfo;

/// Response payload listing workers and their current loads.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListWorkersResponse {
    pub workers: Vec<WorkerInfo>,
}
