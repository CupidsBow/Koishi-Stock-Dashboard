pub mod health;
pub mod indicators;
pub mod search;

use axum::Router;

/// Aggregate all controller routers under the `/api` prefix.
///
/// Each controller defines its own route(s) co-located with its handler
/// functions.  This function merges them so `main.rs` only needs one
/// `.nest("/api", controllers::router())` call.
///
/// Returns `Router` — state is injected in `main.rs` via `.with_state()`.
pub fn router() -> Router {
  Router::new()
    .merge(health::router())
    .merge(indicators::router())
    .merge(search::router())
}
