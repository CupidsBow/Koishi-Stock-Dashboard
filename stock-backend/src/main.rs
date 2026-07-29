mod api;
mod indicators;
mod stock;

use axum::{Router, routing::get};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let app = Router::new()
    .route("/api/health", get(api::health))
    .route("/api/candles", get(api::get_candles))
    .route("/api/minutes", get(api::get_minutes))
    .route("/api/indicators", get(api::get_indicators))
    .route("/api/search", get(api::search_stocks))
    .fallback_service(ServeDir::new("../frontend/dist"))
    .layer(CorsLayer::permissive());

  let addr = "0.0.0.0:3000";
  println!("🚀 Stock Dashboard backend listening on http://{}", addr);

  let listener = tokio::net::TcpListener::bind(addr).await?;
  axum::serve(listener, app).await?;

  Ok(())
}