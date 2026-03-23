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

/// Role a user holds within the API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Consumer,
}

/// Which authentication backend validates a user's password.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    Local,
    Ldap,
}

/// Single entry in `users.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct UserEntry {
    pub username: String,
    pub auth: AuthMethod,
    /// Argon2id hash — required when `auth = "local"`.
    pub password_hash: Option<String>,
    /// LDAP UID substituted into the bind-DN template — required when `auth = "ldap"`.
    pub ldap_uid: Option<String>,
    pub role: Role,
}

/// Top-level shape of `users.json`.
#[derive(Debug, Deserialize)]
pub struct UsersConfig {
    pub users: Vec<UserEntry>,
}
