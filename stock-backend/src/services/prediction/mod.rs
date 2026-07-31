//! Prediction module — orchestrate Stages 1–3 of the pipeline.
//!
//! `compute_alpha_scores()` is the high-level entry point:
//!   candles → raw factors → IC evaluation → normalization → IC-weighted synthesis
//!
//! Returns the complete prediction output: factor evaluations + alpha scores.

use anyhow::Result;

use crate::models::{AlphaScore, FactorDef, FactorEval};

use crate::services::expression::compute_raw_factors;
use crate::services::evaluation::{evaluate_all, rolling_ic};

pub mod normalize;
pub mod synthesize;

use normalize::normalize_factors;
use synthesize::synthesize;

/// Compute alpha scores from candles and factor definitions.
///
/// This is the primary prediction API — it runs Stages 1–3 and returns both
/// the factor evaluations (for transparency) and the composite alpha scores
/// (for decision-making).
///
/// # Arguments
/// * `candles` — OHLCV data
/// * `factor_defs` — factor definitions (names + expressions + categories)
/// * `ic_window` — rolling IC window size (e.g. 60)
/// * `forward_period` — look-ahead period for IC target (e.g. 5)
///
/// # Returns
/// `(Vec<FactorEval>, Vec<Option<AlphaScore>>)` — evaluations and scores
/// aligned on the same time axis.
pub fn compute_alpha_scores(
    candles: &[crate::models::Candle],
    factor_defs: &[FactorDef],
    ic_window: usize,
    forward_period: usize,
) -> Result<(Vec<FactorEval>, Vec<Option<AlphaScore>>)> {
    // Stage 1: Raw factor computation
    let raw_df = compute_raw_factors(candles, factor_defs)?;

    // Extract close prices for IC target computation
    let prices: Vec<f64> = candles.iter().map(|c| c.close).collect();

    // Stage 2: Rolling IC → factor evaluation
    let ic_map = rolling_ic(&raw_df, &prices, ic_window, forward_period)?;
    let evals = evaluate_all(&ic_map, ic_window);

    // Build IC weight map for direction auto-detection
    let ic_weight_map: std::collections::HashMap<String, f64> = evals
        .iter()
        .map(|e| (e.name.clone(), e.ic_mean))
        .collect();

    // Stage 3: Normalize → synthesize
    let norm_df = normalize_factors(&raw_df, factor_defs, &ic_weight_map)?;
    let scores = synthesize(&norm_df, &evals, factor_defs)?;

    Ok((evals, scores))
}