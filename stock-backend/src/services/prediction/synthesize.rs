//! IC-weighted synthesis: z-scored factors → AlphaScore.
//!
//! This is the core prediction step. Each factor's z-score is multiplied by
//! its IC-derived weight, then all weighted scores are summed within each
//! category (momentum/volatility/volume/trend) and across all categories.
//!
//! The `total` field is the model's estimate of expected forward N-day excess
//! return strength. A higher total = stronger bullish expectation.

use anyhow::{Context, Result};
use polars::prelude::*;

use crate::models::{AlphaScore, FactorDef, FactorEval};

/// Synthesize IC-weighted alpha scores from normalized factor values.
///
/// # Arguments
/// * `norm_df` — DataFrame with time + normalized (z-scored, direction-adjusted) factor columns
/// * `evals` — factor evaluations (contain per-factor weights)
/// * `factor_defs` — factor definitions (factor → category mapping)
///
/// # Returns
/// `Vec<Option<AlphaScore>>` — one entry per row in norm_df, None where
/// the total score is effectively zero (all weights zero) or where any
/// required factor column is missing.
pub fn synthesize(
    norm_df: &DataFrame,
    evals: &[FactorEval],
    factor_defs: &[FactorDef],
) -> Result<Vec<Option<AlphaScore>>> {
    let n = norm_df.height();

    // Build lookup maps
    let weight_map: std::collections::HashMap<&str, f64> = evals
        .iter()
        .map(|e| (e.name.as_str(), e.weight))
        .collect();

    let category_map: std::collections::HashMap<&str, &str> = factor_defs
        .iter()
        .map(|f| (f.name.as_str(), f.category.as_str()))
        .collect();

    // Collect per-factor z-score series
    let factor_cols: Vec<String> = norm_df
        .get_column_names()
        .iter()
        .filter(|c| c.as_str() != "time")
        .map(|c| c.to_string())
        .collect();

    let mut factor_series: Vec<(&str, Vec<Option<f64>>)> = Vec::new();
    for col_name in &factor_cols {
        let series = norm_df
            .column(col_name)
            .with_context(|| format!("missing column {}", col_name))?
            .f64()
            .with_context(|| format!("column {} not f64", col_name))?;

        let vals: Vec<Option<f64>> = (0..n).map(|i| series.get(i)).collect();
        factor_series.push((col_name, vals));
    }

    // Extract time column
    let time_vals: Vec<i64> = norm_df
        .column("time")?
        .i64()
        .context("time column not i64")?
        .into_iter()
        .map(|v| v.unwrap_or(0))
        .collect();

    // Compute per-row scores
    let mut scores: Vec<Option<AlphaScore>> = Vec::with_capacity(n);

    for row in 0..n {
        let mut momentum = 0.0f64;
        let mut volatility = 0.0f64;
        let mut volume = 0.0f64;
        let mut trend = 0.0f64;
        let mut any_valid = false;

        for (col_name, vals) in &factor_series {
            if let Some(z) = vals[row] {
                if !z.is_finite() {
                    continue;
                }
                let weight = weight_map.get(col_name).copied().unwrap_or(0.0);
                if weight == 0.0 {
                    continue;
                }
                let cat = category_map.get(col_name).copied().unwrap_or("other");
                let contribution = weight * z;
                match cat {
                    "momentum" => momentum += contribution,
                    "volatility" => volatility += contribution,
                    "volume" => volume += contribution,
                    "trend" => trend += contribution,
                    _ => {}
                }
                any_valid = true;
            }
        }

        if any_valid {
            let total = momentum + volatility + volume + trend;
            scores.push(Some(AlphaScore {
                time: time_vals[row],
                momentum,
                volatility,
                volume,
                trend,
                total,
            }));
        } else {
            scores.push(None);
        }
    }

    Ok(scores)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FactorEval;

    #[test]
    fn test_empty_weights_gives_none() {
        let df = df!(
            "time" => &[1i64, 2, 3],
            "ret_5d" => &[0.5f64, -0.3, 1.2],
        )
        .unwrap();

        let defs = vec![FactorDef::new("ret_5d", "momentum", "...", 1)];
        let evals = vec![FactorEval {
            name: "ret_5d".into(),
            ic_mean: 0.0,
            ir: 0.0,
            weight: 0.0,
            is_valid: false,
            ic_series: vec![],
        }];

        let scores = synthesize(&df, &evals, &defs).unwrap();
        assert_eq!(scores.len(), 3);
        for s in &scores {
            assert!(s.is_none(), "should be None when all weights are 0");
        }
    }

    #[test]
    fn test_positive_weight_gives_scores() {
        let df = df!(
            "time" => &[1i64, 2],
            "ret_5d" => &[1.0f64, -1.0],
        )
        .unwrap();

        let defs = vec![FactorDef::new("ret_5d", "momentum", "...", 1)];
        let evals = vec![FactorEval {
            name: "ret_5d".into(),
            ic_mean: 0.05,
            ir: 0.5,
            weight: 0.05,
            is_valid: true,
            ic_series: vec![],
        }];

        let scores = synthesize(&df, &evals, &defs).unwrap();
        assert_eq!(scores.len(), 2);
        assert!(scores[0].is_some());
        assert!(scores[1].is_some());
        // Both should have momentum = 0.05 * z_score
        assert!((scores[0].as_ref().unwrap().total - 0.05).abs() < 1e-9);
    }
}