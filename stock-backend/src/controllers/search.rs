use axum::{Json, Router, extract::Query, http::StatusCode, response::IntoResponse, routing::get};
use serde::Deserialize;

use crate::services::stock::search_stocks as search_svc;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub keyword: String,
}

/// GET /api/search
async fn search_stocks(Query(params): Query<SearchQuery>) -> impl IntoResponse {
    match search_svc(&params.keyword).await {
        Ok(results) => Json(results).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Search failed: {}", e)})),
        )
            .into_response(),
    }
}

/// Returns the router for this controller.
pub fn router() -> Router {
    Router::new().route("/search", get(search_stocks))
}