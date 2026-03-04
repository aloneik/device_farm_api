use crate::models::domain::DeviceStatus;
use serde::Deserialize;

/// Body for `POST /providers/{provider_id}/devices/{serial}/register`.
/// The provider reports real hardware info for a device it discovered locally.
#[derive(Debug, Deserialize)]
pub struct RegisterDevice {
    pub model: String,
    pub os_version: String,
}

/// Body for `PUT /devices/{serial}/status`.
/// The provider reports the current physical status of a device.
#[derive(Debug, Deserialize)]
pub struct StatusUpdate {
    pub status: DeviceStatus,
}

/// Body for `POST /admin/providers/{provider_id}/devices`.
/// Admin adds a new device entry to a provider's config.
#[derive(Debug, Deserialize)]
pub struct AdminAddDevice {
    pub serial: String,
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
}

/// Body for `PUT /admin/providers/{provider_id}/devices/{serial}`.
/// All fields are optional; only provided ones are applied.
#[derive(Debug, Deserialize)]
pub struct AdminUpdateDevice {
    pub name: Option<String>,
    pub enabled: Option<bool>,
}
