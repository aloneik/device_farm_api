mod handlers;
mod models;
mod sse;
mod state;

use axum::{
    routing::{get, post, put},
    Router,
};
use handlers::{
    add_device, book_device, device_events, get_provider_devices, health, list_all_devices,
    list_devices, list_providers, provider_connect, provider_heartbeat, register_device,
    update_device, update_device_status,
};
use state::AppState;
use tracing::info;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "device_farm_api=info".into()),
        )
        .init();

    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "providers.json".into());
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());

    let state = AppState::new(&config_path);
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    info!("Device Farm API running on http://{bind_addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
    info!("Server stopped");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { info!("Received Ctrl+C, shutting down..."); },
        _ = terminate => { info!("Received SIGTERM, shutting down..."); },
    }
}

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        // Consumer-facing
        .route("/devices", get(list_devices))
        .route("/devices/events", get(device_events))
        .route("/devices/{serial}/book", post(book_device))
        // Provider-facing
        .route("/devices/{serial}/status", put(update_device_status))
        .route("/providers/{provider_id}/devices/{serial}/register", post(register_device))
        .route("/providers", get(list_providers))
        .route("/providers/{id}/devices", get(get_provider_devices))
        .route("/providers/{id}/connect", get(provider_connect))
        .route("/providers/{id}/heartbeat", post(provider_heartbeat))
        // Admin-facing
        .route("/admin/devices", get(list_all_devices))
        .route("/admin/providers/{provider_id}/devices", post(add_device))
        .route("/admin/providers/{provider_id}/devices/{serial}", put(update_device))
        .with_state(state)
}
