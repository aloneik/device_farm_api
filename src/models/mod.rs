pub mod domain;
pub mod requests;
pub mod responses;

// Convenience re-exports so callers can use `models::Device` etc.
pub use domain::{Device, DeviceStatus, ProviderConfig, ProviderDeviceConfig};
pub use requests::{AdminAddDevice, AdminUpdateDevice, RegisterDevice, StatusUpdate};
pub use responses::{AdminDeviceResponse, DeviceEvent, DeviceResponse, ProviderConfigEntry};
