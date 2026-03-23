use crate::models::{ProviderConfigEntry, ProviderDeviceConfig};
use crate::models::responses::ProviderHeartbeatResponse;
use crate::sse::device_status_stream;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::{stream, StreamExt};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

pub async fn list_providers(State(state): State<AppState>) -> axum::Json<Vec<String>> {
    let providers = state.providers.read().unwrap();
    let ids: Vec<String> = providers.keys().cloned().collect();
    info!(count = ids.len(), "Listing providers");
    axum::Json(ids)
}

pub async fn get_provider_devices(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<axum::Json<Vec<ProviderDeviceConfig>>, StatusCode> {
    let providers = state.providers.read().unwrap();
    providers
        .get(&id)
        .cloned()
        .map(axum::Json)
        .ok_or_else(|| {
            warn!(id = %id, "Provider not found");
            StatusCode::NOT_FOUND
        })
}

pub async fn provider_connect(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<
    Sse<impl futures_util::Stream<Item = Result<Event, std::convert::Infallible>>>,
    StatusCode,
> {
    // Resolve the enabled devices for this provider, mapped to the response type
    let config_entries: Vec<ProviderConfigEntry> = {
        let providers = state.providers.read().unwrap();
        providers
            .get(&id)
            .cloned()
            .ok_or_else(|| {
                warn!(id = %id, "Unknown provider attempted to connect");
                StatusCode::NOT_FOUND
            })?
            .iter()
            .filter(|d| d.enabled)
            .map(ProviderConfigEntry::from)
            .collect()
    };

    info!(
        id = %id,
        enabled = config_entries.len(),
        "Provider connected via SSE"
    );

    // First event: send enabled device list so provider knows what to look for
    let config_event = Event::default()
        .event("provider-config")
        .json_data(&config_entries)
        .unwrap();

    let config_stream = stream::once(async move { Ok(config_event) });
    let status_stream = device_status_stream(state.tx.subscribe());

    Ok(Sse::new(config_stream.chain(status_stream)).keep_alive(KeepAlive::default()))
}

pub async fn provider_heartbeat(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<axum::Json<ProviderHeartbeatResponse>, StatusCode> {
    {
        let providers = state.providers.read().unwrap();
        if !providers.contains_key(&id) {
            warn!(id = %id, "Heartbeat from unknown provider");
            return Err(StatusCode::NOT_FOUND);
        }
    }

    let now = SystemTime::now();
    let received_at = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    state.heartbeats.write().unwrap().insert(id.clone(), now);
    info!(id = %id, received_at, "Provider heartbeat received");

    Ok(axum::Json(ProviderHeartbeatResponse { provider_id: id, received_at }))
}
