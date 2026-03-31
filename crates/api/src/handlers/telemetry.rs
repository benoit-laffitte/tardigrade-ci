use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
};
use std::time::Duration;
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

use crate::{ApiState, DeadLetterBuildsResponse, RuntimeMetricsResponse};

/// Streams live operational events to dashboard clients using SSE.
pub(crate) async fn events(
    State(state): State<ApiState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    // BroadcastStream may drop lagging messages; dashboard treats this as best-effort live feed.
    let stream =
        BroadcastStream::new(state.service.subscribe_events()).filter_map(|msg| match msg {
            Ok(event) => {
                let data = serde_json::to_string(&event).ok()?;
                Some(Ok(Event::default().data(data)))
            }
            Err(_) => None,
        });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

/// Returns current reliability metrics snapshot.
pub(crate) async fn metrics(
    State(state): State<ApiState>,
) -> (StatusCode, Json<RuntimeMetricsResponse>) {
    (StatusCode::OK, Json(state.service.metrics_snapshot()))
}

/// Returns build records currently tagged as dead-letter.
pub(crate) async fn dead_letter_builds(
    State(state): State<ApiState>,
) -> Result<(StatusCode, Json<DeadLetterBuildsResponse>), StatusCode> {
    let builds = state
        .service
        .list_dead_letter_builds()
        .await
        .map_err(|e| e.status_code())?;
    Ok((StatusCode::OK, Json(DeadLetterBuildsResponse { builds })))
}
