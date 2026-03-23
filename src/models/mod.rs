pub mod domain;
pub mod requests;
pub mod responses;

// Convenience re-exports so callers can use `models::Device` etc.
pub use domain::{AuthMethod, Device, DeviceStatus, ProviderConfig, ProviderDeviceConfig, Role, UserEntry, UsersConfig};
pub use requests::{AdminAddDevice, AdminUpdateDevice, LoginRequest, RegisterDevice, StatusUpdate};
pub use responses::{AdminDeviceResponse, DeviceEvent, DeviceResponse, LoginResponse, ProviderConfigEntry};
