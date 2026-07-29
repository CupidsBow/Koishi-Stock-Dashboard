use axum::{Json, extract::Query, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::indicators;
use crate::stock;

#[derive(Debug, Serialize)]
pub struct Candle {
  pub time: i64,
  pub open: f64,
  pub high: f64,
  pub low: f64,
  pub close: f64,
  pub volume: f64,
}

#[derive(Debug, Serialize)]
pub struct StockInfo {
  pub symbol: String,
  pub name: String,
  pub market: String,
}

#[derive(Debug, Deserialize)]
pub struct StockQuery {
  pub symbol: String,
  #[serde(default = "default_days")]
  pub days: usize,
}

fn default_days() -> usize {
  400
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
  pub keyword: String,
}

#[derive(Debug, Serialize)]
pub struct IndicatorsResponse {
  pub candles: Vec<Candle>,
  pub bollinger: Vec<Option<indicators::BollingerPoint>>,
  pub keltner: Vec<Option<indicators::KeltnerPoint>>,
  pub macd: Vec<Option<indicators::MacdPoint>>,
  pub kdj: Vec<Option<indicators::KdjPoint>>,
  pub adx: Vec<Option<indicators::AdxPoint>>,
  pub rsi: Vec<Option<f64>>,
  pub regime: String,
  pub signals: Vec<indicators::Signal>,
}

/// GET /api/health
pub async fn health() -> impl IntoResponse {
  Json(serde_json::json!({ "status": "ok", "service": "stock-backend" }))
}

pub async fn get_indicators(Query(params): Query<StockQuery>) -> impl IntoResponse {
  let candles = match stock::fetch_stock_data(&params.symbol, params.days).await {
    Ok(c) => c,
    Err(e) => {
      return (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": format!("Failed: {}", e)})),
      )
        .into_response();
    }
  };
  let bb = indicators::bollinger_bands(&candles, 20);
  let kn = indicators::keltner_channels(&candles);
  let mc = indicators::macd(&candles);
  let kd = indicators::kdj(&candles, 9);
  let ax = indicators::adx(&candles, 14);
  let rs = indicators::rsi(&candles, 2);
  let rg = indicators::market_regime(&ax, &bb, &candles);
  let sig = indicators::compute_signals(&candles, &bb, &kn, &mc, &kd);
  Json(IndicatorsResponse {
    candles,
    bollinger: bb,
    keltner: kn,
    macd: mc,
    kdj: kd,
    adx: ax,
    rsi: rs,
    regime: rg,
    signals: sig,
  })
  .into_response()
}

pub async fn search_stocks(Query(params): Query<SearchQuery>) -> impl IntoResponse {
  match stock::search_stocks(&params.keyword).await {
    Ok(results) => Json(results).into_response(),
    Err(e) => (
      StatusCode::INTERNAL_SERVER_ERROR,
      Json(serde_json::json!({"error": format!("Search failed: {}", e)})),
    )
      .into_response(),
  }
}
