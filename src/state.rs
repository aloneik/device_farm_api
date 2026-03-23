use crate::models::{Device, DeviceEvent, DeviceStatus, ProviderConfig, ProviderDeviceConfig};
use std::{
    collections::HashMap,
    fs,
    sync::{Arc, RwLock},
    time::SystemTime,
};
use tokio::sync::broadcast;
use tracing::info;

pub type DeviceDb = Arc<RwLock<Vec<Device>>>;
// provider_id -> list of per-device config entries
pub type ProviderDb = Arc<RwLock<HashMap<String, Vec<ProviderDeviceConfig>>>>;
// provider_id -> last heartbeat timestamp
pub type HeartbeatDb = Arc<RwLock<HashMap<String, SystemTime>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: DeviceDb,
    pub providers: ProviderDb,
    pub heartbeats: HeartbeatDb,
    pub tx: broadcast::Sender<DeviceEvent>,
}

impl AppState {
    pub fn new(config_path: &str) -> Self {
        let raw = fs::read_to_string(config_path)
            .unwrap_or_else(|_| panic!("Cannot read {config_path}"));
        let configs: Vec<ProviderConfig> = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("Failed to parse {config_path}: {e}"));

        let mut provider_map: HashMap<String, Vec<ProviderDeviceConfig>> = HashMap::new();
        // Pre-seed all enabled devices as Offline stubs so the frontend always
        // sees the full expected inventory even before providers connect.
        let mut all_devices: Vec<Device> = Vec::new();

        for cfg in &configs {
            let enabled: Vec<&ProviderDeviceConfig> =
                cfg.devices.iter().filter(|d| d.enabled).collect();

            info!(
                id = %cfg.id,
                total = cfg.devices.len(),
                enabled = enabled.len(),
                "Loaded provider config"
            );

            for dev_cfg in &enabled {
                all_devices.push(Device {
                    serial: dev_cfg.serial.clone(),
                    // Use the config name as a placeholder until the provider
                    // registers the device with real hardware info.
                    model: dev_cfg.name.clone(),
                    os_version: String::new(),
                    status: DeviceStatus::Offline,
                    provider_id: cfg.id.clone(),
                });
            }

            provider_map.insert(cfg.id.clone(), cfg.devices.clone());
        }

        info!(total_devices = all_devices.len(), "Pre-seeded device inventory from config");

        let (tx, _) = broadcast::channel(64);

        Self {
            db: Arc::new(RwLock::new(all_devices)),
            providers: Arc::new(RwLock::new(provider_map)),
            heartbeats: Arc::new(RwLock::new(HashMap::new())),
            tx,
        }
    }
}
