use axum::{Json, Router, response::IntoResponse, routing::get};

/// GET /api/health
async fn health() -> impl IntoResponse {
  Json(serde_json::json!({ "status": "ok", "service": "stock-backend" }))
}

/// Returns the router for this controller.
pub fn router() -> Router {
  Router::new().route("/health", get(health))
}
