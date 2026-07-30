mod controllers;
mod models;
mod services;

use axum::Extension;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::services::db::{ensure_cache_table, init_pool, spawn_health_check};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let pool = init_pool().await?;

  // Ensure cache table exists before handling requests
  ensure_cache_table(&pool).await?;

  // Background task: ping DB every 30 seconds
  spawn_health_check(pool.clone(), 30);

  let app = Router::new()
    .nest("/api", controllers::router())
    .fallback_service(ServeDir::new("../frontend/dist"))
    .layer(CorsLayer::permissive())
    .layer(Extension(pool));

  let addr = "0.0.0.0:3000";
  println!("🚀 Stock Dashboard backend listening on http://{}", addr);

  let listener = tokio::net::TcpListener::bind(addr).await?;
  axum::serve(listener, app).await?;

  Ok(())
}
