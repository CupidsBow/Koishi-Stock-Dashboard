use axum::{Json, Router, extract::Query, http::StatusCode, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};

use crate::models::{AdxPoint, BollingerPoint, Candle, KdjPoint, KeltnerPoint, MacdPoint, Signal};
use crate::services::fetch_stock_data;
use crate::services::indicators::{adx, bollinger_bands, compute_signals, kdj, keltner_channels, macd, market_regime, rsi};

#[derive(Debug, Deserialize)]
pub struct StockQuery {
    pub symbol: String,
    #[serde(default = "default_days")]
    pub days: usize,
}

fn default_days() -> usize {
    400
}

#[derive(Debug, Serialize)]
pub struct IndicatorsResponse {
    pub candles: Vec<Candle>,
    pub bollinger: Vec<Option<BollingerPoint>>,
    pub keltner: Vec<Option<KeltnerPoint>>,
    pub macd: Vec<Option<MacdPoint>>,
    pub kdj: Vec<Option<KdjPoint>>,
    pub adx: Vec<Option<AdxPoint>>,
    pub rsi: Vec<Option<f64>>,
    pub regime: String,
    pub signals: Vec<Signal>,
}

/// GET /api/indicators
async fn get_indicators(Query(params): Query<StockQuery>) -> impl IntoResponse {
    let candles = match fetch_stock_data(&params.symbol, params.days).await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed: {}", e)})),
            )
                .into_response();
        }
    };
    let bb = bollinger_bands(&candles, 20);
    let kn = keltner_channels(&candles);
    let mc = macd(&candles);
    let kd = kdj(&candles, 9);
    let ax = adx(&candles, 14);
    let rs = rsi(&candles, 2);
    let rg = market_regime(&ax, &bb, &candles);
    let sig = compute_signals(&candles, &bb, &kn, &mc, &kd);
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

/// Returns the router for this controller.
pub fn router() -> Router {
    Router::new().route("/indicators", get(get_indicators))
}