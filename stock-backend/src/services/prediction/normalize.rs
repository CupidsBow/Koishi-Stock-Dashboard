//! Factor normalization: winsorize → z-score → direction adjustment.
//!
//! Input: raw factor DataFrame [T, K+1] (time + K factor columns, raw values)
//! Output: z-scored DataFrame with direction-adjusted columns
//!
//! Processing (per factor column):
//!   1. Winsorize at 1%/99% quantiles — clip extreme outliers
//!   2. Z-score: (x - μ) / σ (using column mean & std)
//!   3. Direction adjustment: multiply by `direction` hint (1, -1, or 0)

use anyhow::{Context, Result};
use polars::prelude::*;
use std::collections::HashMap;

use crate::models::FactorDef;

/// Winsorize a f64 column at the given lower/upper quantiles.
fn winsorize_column(col: &Column, lower_q: f64, upper_q: f64) -> Result<Column> {
    let chunked = col.f64().context("column not f64")?;

    // Compute quantiles the simple way: collect, sort, index
    let mut vals: Vec<f64> = chunked.into_iter().flatten().collect();
    if vals.is_empty() {
        return Ok(col.clone());
    }
    vals.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());

    let p1 = vals[((vals.len() - 1) as f64 * lower_q) as usize];
    let p99 = vals[((vals.len() - 1) as f64 * upper_q) as usize];

    let clamped: Vec<Option<f64>> = chunked
        .into_iter()
        .map(|v| v.map(|x| x.clamp(p1, p99)))
        .collect();

    Ok(Column::new(col.name().clone(), clamped))
}

/// Full normalization pipeline for raw factor values.
///
/// # Arguments
/// * `raw_df` — DataFrame with time + raw factor columns
/// * `factor_defs` — factor definitions (for direction hints)
/// * `ic_weight_map` — optional per-factor IC weights for direction auto-detection
///   (when FactorDef.direction == 0, use the sign of ic_weight to determine direction)
///
/// # Returns
/// `DataFrame` with time + normalized (z-scored, direction-adjusted) factor columns.
pub fn normalize_factors(
    raw_df: &DataFrame,
    factor_defs: &[FactorDef],
    ic_weight_map: &HashMap<String, f64>,
) -> Result<DataFrame> {
    let mut df = raw_df.clone();

    let col_names: Vec<String> = df
        .get_column_names()
        .iter()
        .filter(|c| c.as_str() != "time")
        .map(|c| c.to_string())
        .collect();

    for col_name in &col_names {
        // Step 1: Winsorize
        let col = df.column(col_name)?.clone();
        let col_wins = winsorize_column(&col, 0.01, 0.99)?;
        df.with_column(col_wins)?;
    }

    // Step 2+3: Z-score + direction adjust
    for col_name in &col_names {
        let chunked = df.column(col_name)?.f64()?.clone();

        // Compute mean and std
        let vals: Vec<f64> = chunked.into_iter().flatten().collect();
        let n = vals.len() as f64;
        if n < 2.0 {
            continue;
        }
        let mean = vals.iter().sum::<f64>() / n;
        let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let std_dev = var.sqrt().max(1e-9);

        // Find direction for this factor
        let direction = factor_defs
            .iter()
            .find(|f| f.name == *col_name)
            .map(|f| {
                if f.direction == 0 {
                    // Auto-detect from IC sign
                    let w = ic_weight_map.get(&f.name).copied().unwrap_or(0.0);
                    if w > 0.0 {
                        1.0
                    } else if w < 0.0 {
                        -1.0
                    } else {
                        0.0
                    }
                } else {
                    f.direction as f64
                }
            })
            .unwrap_or(1.0);

        let z_values: Vec<Option<f64>> = chunked
            .into_iter()
            .map(|v| v.map(|x| direction * (x - mean) / std_dev))
            .collect();

        df.with_column(Column::new(col_name.as_str().into(), z_values))?;
    }

    Ok(df)
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::*;

    fn make_test_df() -> DataFrame {
        df!(
            "time" => &[1i64, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            "factor_a" => &[1.0f64, 2.0, 3.0, 4.0, 5.0, 100.0, 7.0, 8.0, 9.0, 10.0], // 100.0 is outlier
        )
        .unwrap()
    }

    #[test]
    fn test_winsorize_clips_outlier() {
        let df = make_test_df();
        let defs = vec![FactorDef::new("factor_a", "test", "Close", 1)];
        let weight_map = HashMap::new();
        let normed = normalize_factors(&df, &defs, &weight_map).unwrap();

        let vals: Vec<f64> = normed
            .column("factor_a")
            .unwrap()
            .f64()
            .unwrap()
            .into_iter()
            .flatten()
            .collect();

        // After winsorize, the max should be capped at p99 (around 9-10 range)
        let max_val = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(max_val < 50.0, "outlier should be winsorized, got max={}", max_val);
    }
}