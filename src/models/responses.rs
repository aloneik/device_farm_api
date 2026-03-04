use crate::models::domain::{Device, DeviceStatus, ProviderDeviceConfig};
use serde::Serialize;

/// HTTP response body for device-related consumer and provider endpoints.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceResponse {
    pub serial: String,
    pub model: String,
    pub os_version: String,
    pub status: DeviceStatus,
    pub provider_id: String,
}

impl From<&Device> for DeviceResponse {
    fn from(d: &Device) -> Self {
        Self {
            serial: d.serial.clone(),
            model: d.model.clone(),
            os_version: d.os_version.clone(),
            status: d.status.clone(),
            provider_id: d.provider_id.clone(),
        }
    }
}

impl From<Device> for DeviceResponse {
    fn from(d: Device) -> Self {
        Self::from(&d)
    }
}

/// Payload broadcast over the SSE channel whenever a device's status changes.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceEvent {
    pub serial: String,
    pub model: String,
    pub status: DeviceStatus,
}

impl DeviceEvent {
    pub fn from_device(device: &Device) -> Self {
        Self {
            serial: device.serial.clone(),
            model: device.model.clone(),
            status: device.status.clone(),
        }
    }
}

/// Entry sent to a provider in the `provider-config` SSE event.
/// Only enabled devices are included.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderConfigEntry {
    pub serial: String,
    pub name: String,
}

impl From<&ProviderDeviceConfig> for ProviderConfigEntry {
    fn from(cfg: &ProviderDeviceConfig) -> Self {
        Self {
            serial: cfg.serial.clone(),
            name: cfg.name.clone(),
        }
    }
}

/// Full device view returned to the admin panel.
/// Merges static config data with live runtime state.
#[derive(Debug, Clone, Serialize)]
pub struct AdminDeviceResponse {
    pub serial: String,
    /// Human-readable label from config.
    pub name: String,
    /// `None` until the provider registers the device.
    pub model: Option<String>,
    pub os_version: Option<String>,
    /// `None` when the device is disabled in config.
    pub status: Option<DeviceStatus>,
    pub provider_id: String,
    pub enabled: bool,
}
