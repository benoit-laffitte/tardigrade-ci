use uuid::Uuid;

/// Builds claim endpoint URL for one worker id.
pub(crate) fn claim_url(server_url: &str, worker_id: &str) -> String {
    format!("{server_url}/workers/{worker_id}/claim")
}

/// Builds completion endpoint URL for one worker/build pair.
pub(crate) fn complete_url(server_url: &str, worker_id: &str, build_id: Uuid) -> String {
    format!("{server_url}/workers/{worker_id}/builds/{build_id}/complete")
}

#[cfg(test)]
mod tests;
