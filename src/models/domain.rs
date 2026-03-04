use serde::{Deserialize, Serialize};

/// The current state of a device. Shared across all layers.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum DeviceStatus {
    Available,
    Busy,
    Offline,
}

/// Live device record stored in the in-memory DB.
/// Populated at runtime when a provider registers a device.
#[derive(Debug, Clone)]
pub struct Device {
    pub serial: String,
    /// Friendly model name reported by the provider (e.g. "Pixel 7").
    pub model: String,
    pub os_version: String,
    pub status: DeviceStatus,
    /// Id of the provider that owns this device.
    pub provider_id: String,
}

/// Top-level entry in `providers.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub devices: Vec<ProviderDeviceConfig>,
}

/// Per-device config entry inside `providers.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDeviceConfig {
    pub serial: String,
    /// Human-readable label used as a placeholder before provider connects.
    pub name: String,
    pub enabled: bool,
}
