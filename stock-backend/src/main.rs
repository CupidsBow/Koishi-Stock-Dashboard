mod models;
mod controllers;
mod services;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::services::db::{AppState, init_pool, spawn_health_check};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = init_pool().await?;

    // Background task: ping DB every 30 seconds
    spawn_health_check(pool.clone(), 30);

    let app = Router::new()
        .with_state(AppState::new(pool))
        .nest("/api", controllers::router())
        .fallback_service(ServeDir::new("../frontend/dist"))
        .layer(CorsLayer::permissive());

    let addr = "0.0.0.0:3000";
    println!("🚀 Stock Dashboard backend listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}