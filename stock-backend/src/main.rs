mod models;
mod controllers;
mod services;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .nest("/api", controllers::router())
        .fallback_service(ServeDir::new("../frontend/dist"))
        .layer(CorsLayer::permissive());

    let addr = "0.0.0.0:3000";
    println!("🚀 Stock Dashboard backend listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}