use axum::{routing::get, Router};

pub fn routes() -> Router {
    Router::new().route("/health", get(health))
}

/// Health check endpoint
#[utoipa::path(get, path = "/health", responses((status = 200, description = "Ok")))]
async fn health() -> String {
    "Ok".to_string()
}
