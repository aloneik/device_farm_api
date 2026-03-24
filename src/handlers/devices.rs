use crate::auth::jwt::Claims;
use crate::models::{Device, DeviceEvent, DeviceResponse, DeviceStatus, RegisterDevice, StatusUpdate};
use crate::sse::device_status_stream;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use tracing::{info, warn};

pub async fn list_devices(
    _claims: Claims,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let devices = state.db.read().unwrap();
    info!(count = devices.len(), "Listing devices");
    let response: Vec<DeviceResponse> = devices.iter().map(DeviceResponse::from).collect();
    Json(response)
}

/// Called by a provider to register a device it discovered locally.
/// Only serials that are enabled in the provider's config are accepted.
pub async fn register_device(
    Path((provider_id, serial)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(payload): Json<RegisterDevice>,
) -> Result<Json<DeviceResponse>, StatusCode> {
    // Verify the provider exists and has this serial enabled
    let providers = state.providers.read().unwrap();
    let provider_devices = providers.get(&provider_id).ok_or_else(|| {
        warn!(provider_id = %provider_id, "Unknown provider tried to register device");
        StatusCode::NOT_FOUND
    })?;

    let is_enabled = provider_devices
        .iter()
        .any(|d| d.serial == serial && d.enabled);

    if !is_enabled {
        warn!(
            serial = %serial,
            provider_id = %provider_id,
            "Device serial not enabled for this provider"
        );
        return Err(StatusCode::FORBIDDEN);
    }
    drop(providers); // release read lock before acquiring write lock

    let mut devices = state.db.write().unwrap();

    // If already registered (re-connect), update model/os_version and set Available
    if let Some(existing) = devices.iter_mut().find(|d| d.serial == serial) {
        existing.model = payload.model;
        existing.os_version = payload.os_version;
        existing.status = DeviceStatus::Available;
        info!(serial = %serial, provider_id = %provider_id, "Device re-registered");
        let _ = state.tx.send(DeviceEvent::from_device(existing));
        return Ok(Json(DeviceResponse::from(existing as &Device)));
    }

    let device = Device {
        serial: serial.clone(),
        model: payload.model,
        os_version: payload.os_version,
        status: DeviceStatus::Available,
        provider_id: provider_id.clone(),
    };

    info!(
        serial = %device.serial,
        model = %device.model,
        provider_id = %provider_id,
        "Device registered"
    );
    let _ = state.tx.send(DeviceEvent::from_device(&device));
    let response = DeviceResponse::from(&device);
    devices.push(device);
    Ok(Json(response))
}

pub async fn book_device(
    _claims: Claims,
    Path(serial): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<DeviceResponse>, StatusCode> {
    let mut devices = state.db.write().unwrap();

    match devices.iter_mut().find(|d| d.serial == serial) {
        None => {
            warn!(serial = %serial, "Device not found");
            Err(StatusCode::NOT_FOUND)
        }
        Some(device) if device.status == DeviceStatus::Offline => {
            warn!(serial = %device.serial, model = %device.model, "Device is offline, cannot book");
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
        Some(device) if device.status == DeviceStatus::Busy => {
            warn!(serial = %device.serial, model = %device.model, "Device is already busy");
            Err(StatusCode::CONFLICT)
        }
        Some(device) => {
            device.status = DeviceStatus::Busy;
            info!(serial = %device.serial, model = %device.model, "Device booked");
            let _ = state.tx.send(DeviceEvent::from_device(device));
            Ok(Json(DeviceResponse::from(device as &Device)))
        }
    }
}

pub async fn release_device(
    _claims: Claims,
    Path(serial): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<DeviceResponse>, StatusCode> {
    let mut devices = state.db.write().unwrap();

    match devices.iter_mut().find(|d| d.serial == serial) {
        None => {
            warn!(serial = %serial, "Device not found");
            Err(StatusCode::NOT_FOUND)
        }
        Some(device) if device.status == DeviceStatus::Offline => {
            warn!(serial = %device.serial, model = %device.model, "Device is offline, cannot release");
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
        Some(device) if device.status == DeviceStatus::Available => {
            warn!(serial = %device.serial, model = %device.model, "Device is already available");
            Err(StatusCode::CONFLICT)
        }
        Some(device) => {
            device.status = DeviceStatus::Available;
            info!(serial = %device.serial, model = %device.model, "Device released");
            let _ = state.tx.send(DeviceEvent::from_device(device));
            Ok(Json(DeviceResponse::from(device as &Device)))
        }
    }
}

pub async fn update_device_status(
    Path(serial): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<StatusUpdate>,
) -> Result<Json<DeviceResponse>, StatusCode> {
    let mut devices = state.db.write().unwrap();

    let device = devices
        .iter_mut()
        .find(|d| d.serial == serial)
        .ok_or_else(|| {
            warn!(serial = %serial, "Device not found for status update");
            StatusCode::NOT_FOUND
        })?;

    let old_status = device.status.clone();
    device.status = payload.status;
    info!(
        serial = %device.serial,
        model = %device.model,
        old = ?old_status,
        new = ?device.status,
        "Provider updated device status"
    );
    let _ = state.tx.send(DeviceEvent::from_device(device));
    Ok(Json(DeviceResponse::from(device as &Device)))
}

pub async fn device_events(
    _claims: Claims,
    State(state): State<AppState>,
) -> Sse<impl futures_util::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>
{
    info!("SSE client connected to /devices/events");
    Sse::new(device_status_stream(state.tx.subscribe())).keep_alive(KeepAlive::default())
}

