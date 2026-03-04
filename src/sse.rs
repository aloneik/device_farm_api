use crate::models::DeviceEvent;
use axum::response::sse::Event;
use futures_util::StreamExt;
use std::convert::Infallible;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

/// Converts a broadcast receiver into an SSE stream of `device-status` events.
/// Lagged/dropped messages are silently skipped.
pub fn device_status_stream(
    rx: broadcast::Receiver<DeviceEvent>,
) -> impl futures_util::Stream<Item = Result<Event, Infallible>> {
    BroadcastStream::new(rx).filter_map(|result| async move {
        result.ok().map(|event| {
            let data = serde_json::to_string(&event).unwrap_or_default();
            Ok(Event::default().event("device-status").data(data))
        })
    })
}
