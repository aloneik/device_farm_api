use crate::auth::jwt::Claims;
use crate::models::{
    AdminAddDevice, AdminDeviceResponse, AdminUpdateDevice, Device, DeviceEvent, DeviceStatus,
    ProviderDeviceConfig, Role,
};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use tracing::{info, warn};

/// `GET /admin/devices`
///
/// Returns every device from every provider's config, including disabled ones.
/// Live runtime data (model, os_version, status) is overlaid where available.
pub async fn list_all_devices(
    claims: Claims,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    if claims.role != Role::Admin {
        return Err(StatusCode::FORBIDDEN);
    }
    let providers = state.providers.read().unwrap();
    let db = state.db.read().unwrap();

    let mut result: Vec<AdminDeviceResponse> = Vec::new();

    for (provider_id, devices) in providers.iter() {
        for cfg in devices {
            let live: Option<&Device> = db.iter().find(|d| d.serial == cfg.serial);
            result.push(AdminDeviceResponse {
                serial: cfg.serial.clone(),
                name: cfg.name.clone(),
                model: live.map(|d| d.model.clone()),
                os_version: live.map(|d| d.os_version.clone()),
                // Disabled devices have no meaningful live status from the frontend's POV
                status: if cfg.enabled {
                    Some(live.map_or(DeviceStatus::Offline, |d| d.status.clone()))
                } else {
                    None
                },
                provider_id: provider_id.clone(),
                enabled: cfg.enabled,
            });
        }
    }

    info!(count = result.len(), "Admin: listing all devices");
    Ok(Json(result))
}

/// `POST /admin/providers/{provider_id}/devices`
///
/// Adds a new device entry to a provider's config.
/// If `enabled` is true, an `Offline` stub is immediately inserted into the live DB
/// so the frontend can see it before the provider connects.
pub async fn add_device(
    claims: Claims,
    Path(provider_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<AdminAddDevice>,
) -> Result<Json<AdminDeviceResponse>, StatusCode> {
    if claims.role != Role::Admin {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut providers = state.providers.write().unwrap();

    let provider_devices = providers.get_mut(&provider_id).ok_or_else(|| {
        warn!(provider_id = %provider_id, "Admin: provider not found");
        StatusCode::NOT_FOUND
    })?;

    if provider_devices.iter().any(|d| d.serial == payload.serial) {
        warn!(
            serial = %payload.serial,
            provider_id = %provider_id,
            "Admin: device serial already exists for this provider"
        );
        return Err(StatusCode::CONFLICT);
    }

    provider_devices.push(ProviderDeviceConfig {
        serial: payload.serial.clone(),
        name: payload.name.clone(),
        enabled: payload.enabled,
    });
    drop(providers); // release write lock before touching DB

    if payload.enabled {
        let mut db = state.db.write().unwrap();
        db.push(Device {
            serial: payload.serial.clone(),
            model: payload.name.clone(),
            os_version: String::new(),
            status: DeviceStatus::Offline,
            provider_id: provider_id.clone(),
        });
        info!(
            serial = %payload.serial,
            provider_id = %provider_id,
            "Admin: device added and seeded as Offline"
        );
    } else {
        info!(
            serial = %payload.serial,
            provider_id = %provider_id,
            "Admin: disabled device added to config"
        );
    }

    Ok(Json(AdminDeviceResponse {
        serial: payload.serial,
        name: payload.name,
        model: None,
        os_version: None,
        status: if payload.enabled {
            Some(DeviceStatus::Offline)
        } else {
            None
        },
        provider_id,
        enabled: payload.enabled,
    }))
}

/// `PUT /admin/providers/{provider_id}/devices/{serial}`
///
/// Updates a device's config entry (`name` and/or `enabled`).
///
/// - Toggling `enabled` **true → false**: removes the device from the live DB and
///   broadcasts an `Offline` event so consumers and providers are notified.
/// - Toggling `enabled` **false → true**: inserts an `Offline` stub into the live DB.
pub async fn update_device(
    claims: Claims,
    Path((provider_id, serial)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(payload): Json<AdminUpdateDevice>,
) -> Result<Json<AdminDeviceResponse>, StatusCode> {
    if claims.role != Role::Admin {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut providers = state.providers.write().unwrap();

    let provider_devices = providers.get_mut(&provider_id).ok_or_else(|| {
        warn!(provider_id = %provider_id, "Admin: provider not found");
        StatusCode::NOT_FOUND
    })?;

    let cfg = provider_devices
        .iter_mut()
        .find(|d| d.serial == serial)
        .ok_or_else(|| {
            warn!(serial = %serial, provider_id = %provider_id, "Admin: device not found in config");
            StatusCode::NOT_FOUND
        })?;

    let was_enabled = cfg.enabled;

    if let Some(name) = payload.name {
        cfg.name = name;
    }
    if let Some(enabled) = payload.enabled {
        cfg.enabled = enabled;
    }

    let now_enabled = cfg.enabled;
    let cfg_name = cfg.name.clone();
    drop(providers); // release write lock before touching DB

    match (was_enabled, now_enabled) {
        (true, false) => {
            // Remove from live DB and notify subscribers
            let mut db = state.db.write().unwrap();
            if let Some(pos) = db.iter().position(|d| d.serial == serial) {
                let removed = db.remove(pos);
                let _ = state.tx.send(DeviceEvent {
                    serial: removed.serial.clone(),
                    model: removed.model.clone(),
                    status: DeviceStatus::Offline,
                });
                info!(serial = %serial, provider_id = %provider_id, "Admin: device disabled and removed from live DB");
            }
        }
        (false, true) => {
            // Add Offline stub to live DB
            let mut db = state.db.write().unwrap();
            if !db.iter().any(|d| d.serial == serial) {
                db.push(Device {
                    serial: serial.clone(),
                    model: cfg_name.clone(),
                    os_version: String::new(),
                    status: DeviceStatus::Offline,
                    provider_id: provider_id.clone(),
                });
                info!(serial = %serial, provider_id = %provider_id, "Admin: device enabled and seeded as Offline");
            }
        }
        _ => {} // enabled state unchanged — no DB adjustment needed
    }

    // Build the response from current state
    let db = state.db.read().unwrap();
    let live = db.iter().find(|d| d.serial == serial);

    let providers = state.providers.read().unwrap();
    let cfg = providers
        .get(&provider_id)
        .and_then(|devs| devs.iter().find(|d| d.serial == serial))
        .unwrap(); // we just modified it, it exists

    Ok(Json(AdminDeviceResponse {
        serial: serial.clone(),
        name: cfg.name.clone(),
        model: live.map(|d| d.model.clone()),
        os_version: live.map(|d| d.os_version.clone()),
        status: if cfg.enabled {
            Some(live.map_or(DeviceStatus::Offline, |d| d.status.clone()))
        } else {
            None
        },
        provider_id,
        enabled: cfg.enabled,
    }))
}
