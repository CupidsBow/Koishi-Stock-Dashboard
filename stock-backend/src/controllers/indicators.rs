use axum::{
  Extension, Json, Router, extract::Query, http::StatusCode, response::IntoResponse, routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::models::{
  AdxPoint, AlphaScore, BollingerPoint, Candle, FactorEval, KdjPoint, KeltnerPoint, MacdPoint,
  Signal,
};
use crate::services::cache::get_candles_with_cache;
use crate::services::expression::builtin_factors;
use crate::services::indicators::{
  adx, bollinger_bands, compute_signals, kdj, keltner_channels, macd, market_regime, rsi,
};
use crate::services::prediction::compute_alpha_scores;
use crate::services::signals::{generate_signals, pair_signals, SignalConfig};

// ── Query Parameters ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StockQuery {
  pub symbol: String,
  #[serde(default = "default_days")]
  pub days: usize,
  /// Strategy: "default" (CTA rules) | "factor" (alpha) | "hybrid" (both)
  #[serde(default = "default_strategy")]
  pub strategy: String,
  /// Forward prediction horizon (only relevant for factor/hybrid)
  #[serde(default = "default_forward")]
  pub forward: usize,
  /// Enable quantile-threshold signals
  #[serde(default = "default_true")]
  pub quantile: bool,
  /// Enable score-reversal signals
  #[serde(default = "default_true")]
  pub reversal: bool,
  /// Enable price-score divergence signals
  #[serde(default = "default_true")]
  pub divergence: bool,
}

fn default_days() -> usize { 400 }
fn default_strategy() -> String { "default".into() }
fn default_forward() -> usize { 5 }
fn default_true() -> bool { true }

// ── Response ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct IndicatorsResponse {
  // Legacy fields (always present)
  pub candles: Vec<Candle>,
  pub bollinger: Vec<Option<BollingerPoint>>,
  pub keltner: Vec<Option<KeltnerPoint>>,
  pub macd: Vec<Option<MacdPoint>>,
  pub kdj: Vec<Option<KdjPoint>>,
  pub adx: Vec<Option<AdxPoint>>,
  pub rsi: Vec<Option<f64>>,
  pub regime: String,
  pub signals: Vec<Signal>,

  // Factor-model fields (only present for factor/hybrid strategies)
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub factor_evals: Vec<FactorEval>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub factor_scores: Vec<Option<AlphaScore>>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub signals_v2: Vec<Signal>,
}

// ── Handler ──────────────────────────────────────────────────────────────────

/// GET /api/indicators
async fn get_indicators(
  Extension(pool): Extension<PgPool>,
  Query(params): Query<StockQuery>,
) -> impl IntoResponse {
  let candles = match get_candles_with_cache(&pool, &params.symbol, params.days).await {
    Ok(c) => c,
    Err(e) => {
      return (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": format!("Failed: {}", e)})),
      )
        .into_response();
    }
  };

  // ── Legacy CTA indicator computation (always run for backward compat) ──
  let bb = bollinger_bands(&candles, 20);
  let kn = keltner_channels(&candles);
  let mc = macd(&candles);
  let kd = kdj(&candles, 9);
  let ax = adx(&candles, 14);
  let rs = rsi(&candles, 2);
  let rg = market_regime(&ax, &bb, &candles);
  let sig = compute_signals(&candles, &bb, &kn, &mc, &kd);

  // ── Factor-model signals (only for factor/hybrid) ──
  let (factor_evals, factor_scores, mut signals_v2) = match params.strategy.as_str() {
    "factor" | "hybrid" => {
      let ic_window = 60usize.min(params.days / 3);
      let forward = params.forward;

      match compute_and_signal(&candles, ic_window, forward, params.quantile, params.reversal, params.divergence) {
        Ok((evals, scores, sigs)) => (evals, scores, sigs),
        Err(e) => {
          eprintln!("Factor pipeline error for {}: {e}", params.symbol);
          (Vec::new(), Vec::new(), Vec::new())
        }
      }
    }
    _ => (Vec::new(), Vec::new(), Vec::new()),
  };

  // Pair factor buy/sell signals and compute pnl_pct (matching legacy format)
  if !signals_v2.is_empty() {
    signals_v2 = pair_signals(&signals_v2, &candles);
  }

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
    factor_evals,
    factor_scores,
    signals_v2,
  })
  .into_response()
}

// ── Factor pipeline helper ───────────────────────────────────────────────────

fn compute_and_signal(
  candles: &[Candle],
  ic_window: usize,
  forward_period: usize,
  enable_quantile: bool,
  enable_reversal: bool,
  enable_divergence: bool,
) -> anyhow::Result<(Vec<FactorEval>, Vec<Option<AlphaScore>>, Vec<Signal>)> {
  let factor_defs = builtin_factors();
  let (evals, scores) =
    compute_alpha_scores(candles, &factor_defs, ic_window, forward_period)?;

  let config = SignalConfig {
    pct_window: 120,
    divergence_window: 20,
    enable_quantile,
    enable_reversal,
    enable_divergence,
  };
  let sigs = generate_signals(&scores, candles, &config);

  Ok((evals, scores, sigs))
}

/// Returns the router for this controller.
pub fn router() -> Router {
  Router::new().route("/indicators", get(get_indicators))
}