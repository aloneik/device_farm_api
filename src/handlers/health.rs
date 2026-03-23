use crate::models::responses::HealthResponse;
use axum::Json;
use tracing::info;

pub async fn health() -> Json<HealthResponse> {
    info!("Health check");
    Json(HealthResponse { status: "ok" })
}
