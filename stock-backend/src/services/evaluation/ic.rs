//! Rolling Information Coefficient (IC) computation for each factor.
//!
//! IC[t][factor] = Pearson correlation between:
//!   - factor_raw[t - IC_WINDOW .. t]   (window of factor values)
//!   - fwd_ret[t - IC_WINDOW .. t]      (window of forward N-day returns)
//!
//! Forward return: fwd_ret[t] = (close[t+FORWARD_PERIOD] - close[t]) / close[t]

use anyhow::{Context, Result};
use ndarray::Array1;
use polars::prelude::*;
use std::collections::HashMap;

/// Compute rolling IC series for all factor columns.
///
/// # Arguments
/// * `factor_df` — DataFrame with `time` column + one column per factor (raw values)
/// * `prices` — close price array, aligned with factor_df rows
/// * `ic_window` — number of periods for the rolling correlation window (e.g. 60)
/// * `forward_period` — number of periods to look ahead for return (e.g. 5)
///
/// # Returns
/// `HashMap<String, Vec<Option<f64>>>` — factor name → IC series (None where
/// insufficient data to compute correlation).
pub fn rolling_ic(
    factor_df: &DataFrame,
    prices: &[f64],
    ic_window: usize,
    forward_period: usize,
) -> Result<HashMap<String, Vec<Option<f64>>>> {
    let n = factor_df.height();
    // Compute forward returns: fwd_ret[t] = (p[t+F] - p[t]) / p[t]
    let mut fwd_rets: Vec<Option<f64>> = vec![None; n];
    for t in 0..n {
        if t + forward_period < n {
            let p0 = prices[t];
            let p1 = prices[t + forward_period];
            if p0 > 1e-12 {
                fwd_rets[t] = Some((p1 - p0) / p0);
            }
        }
    }

    let factor_cols: Vec<String> = factor_df
        .get_column_names()
        .iter()
        .filter(|c| c.as_str() != "time")
        .map(|c| c.to_string())
        .collect();

    let mut result: HashMap<String, Vec<Option<f64>>> = HashMap::new();

    for col_name in &factor_cols {
        let series = factor_df
            .column(col_name)
            .with_context(|| format!("missing column {}", col_name))?
            .f64()
            .with_context(|| format!("column {} is not f64", col_name))?;

        // Extract raw values (null → None)
        let raw_vals: Vec<Option<f64>> = (0..n).map(|i| series.get(i)).collect();

        let mut ic_series: Vec<Option<f64>> = vec![None; n];

        for t in ic_window..(n - forward_period) {
            let start = t.saturating_sub(ic_window);
            let window_end = t;

            // Collect paired (factor, fwd_ret) for this window
            let mut x_vals = Vec::with_capacity(ic_window);
            let mut y_vals = Vec::with_capacity(ic_window);

            for j in start..window_end {
                if let (Some(x), Some(y)) = (raw_vals[j], fwd_rets[j]) {
                    if x.is_finite() && y.is_finite() {
                        x_vals.push(x);
                        y_vals.push(y);
                    }
                }
            }

            if x_vals.len() < ic_window / 3 {
                // Too few valid data points — skip
                continue;
            }

            let ic = pearson_corr(&x_vals, &y_vals);
            ic_series[t] = Some(ic);
        }

        result.insert(col_name.to_string(), ic_series);
    }

    Ok(result)
}

/// Compute the Pearson correlation coefficient between two slices.
///
/// corr = Σ((x-μx)(y-μy)) / √(Σ(x-μx)² · Σ(y-μy)²)
fn pearson_corr(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    if n < 2.0 {
        return 0.0;
    }

    let mean_x = x.iter().sum::<f64>() / n;
    let mean_y = y.iter().sum::<f64>() / n;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;

    for i in 0..x.len() {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    if denom < 1e-15 {
        0.0
    } else {
        (cov / denom).clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pearson_perfect_positive() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        assert!((pearson_corr(&x, &y) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_pearson_perfect_negative() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 8.0, 6.0, 4.0, 2.0];
        assert!((pearson_corr(&x, &y) + 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_pearson_zero() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![5.0, 5.0, 5.0];
        assert!((pearson_corr(&x, &y) - 0.0).abs() < 1e-9);
    }
}