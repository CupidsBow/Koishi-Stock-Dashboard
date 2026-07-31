//! Convert `[Candle]` → ndarray → raw factor values column-by-column.
//!
//! Instead of compiling factor expressions into Polars lazy Expr trees (which
//! has a steep feature-gating surface in Polars 0.51), this module extracts
//! per-column `Vec<f64>` from a DataFrame and applies pre-defined factor
//! functions. The expression parser in `parser.rs` validates the formula
//! syntax but actual computation uses these hard-coded functions.
//!
//! When the factor registry grows (>20 factors), we can graduate to a more
//! general compile-and-execute approach.

use anyhow::{Context, Result};
use polars::prelude::*;

use crate::models::{Candle, FactorDef};

// ── DataFrame construction ──────────────────────────────────────────────────

/// Build a Polars `DataFrame` from a slice of candles.
pub fn candles_to_df(candles: &[Candle]) -> DataFrame {
    let n = candles.len();
    let mut times = Vec::with_capacity(n);
    let mut opens = Vec::with_capacity(n);
    let mut highs = Vec::with_capacity(n);
    let mut lows = Vec::with_capacity(n);
    let mut closes = Vec::with_capacity(n);
    let mut volumes = Vec::with_capacity(n);

    for c in candles {
        times.push(c.time);
        opens.push(c.open);
        highs.push(c.high);
        lows.push(c.low);
        closes.push(c.close);
        volumes.push(c.volume);
    }

    df!(
        "time"   => &times,
        "open"   => &opens,
        "high"   => &highs,
        "low"    => &lows,
        "close"  => &closes,
        "volume" => &volumes,
    )
    .expect("DataFrame construction from candles should be infallible")
}

// ── Rolling-helpers (operate on `&[f64]`, no Polars dependency) ─────────────

fn rolling_mean(vals: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = vals.len();
    let mut out = vec![None; n];
    if n < period {
        return out;
    }
    let mut sum = vals[..period].iter().sum::<f64>();
    out[period - 1] = Some(sum / period as f64);
    for i in period..n {
        sum += vals[i] - vals[i - period];
        out[i] = Some(sum / period as f64);
    }
    out
}

fn rolling_std(vals: &[f64], period: usize) -> Vec<Option<f64>> {
    let means = rolling_mean(vals, period);
    let n = vals.len();
    let mut out = vec![None; n];
    if n < period {
        return out;
    }
    for i in (period - 1)..n {
        let slice = &vals[i + 1 - period..=i];
        let m = means[i].unwrap();
        let var = slice.iter().map(|x| (x - m).powi(2)).sum::<f64>() / period as f64;
        out[i] = Some(var.sqrt());
    }
    out
}

fn rolling_max(vals: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = vals.len();
    let mut out = vec![None; n];
    if n < period {
        return out;
    }
    for i in (period - 1)..n {
        let mx = vals[i + 1 - period..=i]
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        out[i] = Some(mx);
    }
    out
}

fn rolling_min(vals: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = vals.len();
    let mut out = vec![None; n];
    if n < period {
        return out;
    }
    for i in (period - 1)..n {
        let mn = vals[i + 1 - period..=i]
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        out[i] = Some(mn);
    }
    out
}

fn delta(vals: &[f64], lag: usize) -> Vec<Option<f64>> {
    let n = vals.len();
    let mut out = vec![None; n];
    for i in lag..n {
        out[i] = Some(vals[i] - vals[i - lag]);
    }
    out
}

fn delay(vals: &[f64], lag: usize) -> Vec<Option<f64>> {
    let n = vals.len();
    let mut out = vec![None; n];
    for i in lag..n {
        out[i] = Some(vals[i - lag]);
    }
    out
}

fn abs_vals(vals: &[Option<f64>]) -> Vec<Option<f64>> {
    vals.iter().map(|v| v.map(|x| x.abs())).collect()
}

// ── Factor evaluation (col-name → Vec<Option<f64>>) ─────────────────────────

/// Compute the raw value vector for a single factor expression.
///
/// Supported expressions are the 12 built-in formulas from `registry.rs`.
/// Unknown / complex expressions return an error clarifying the limitation.
fn evaluate_factor_expression(
    close: &[f64],
    high: &[f64],
    low: &[f64],
    volume: &[f64],
    expression: &str,
) -> Result<Vec<Option<f64>>> {
    match expression {
        // Momentum
        s if s.contains("Delta(Close,5)/Delay(Close,5)")
            && s.contains("20")
            && !s.contains("20,") =>
        {
            let d = delta(close, 20);
            let dl = delay(close, 20);
            Ok(zip_div(&d, &dl))
        }
        "Delta(Close,5)/Delay(Close,5)" => {
            let d = delta(close, 5);
            let dl = delay(close, 5);
            Ok(zip_div(&d, &dl))
        }
        "Ts_Mean(Close,12)-Ts_Mean(Close,26)" => {
            let m12 = rolling_mean(close, 12);
            let m26 = rolling_mean(close, 26);
            Ok(zip_sub(&m12, &m26))
        }
        s if s.contains("(Close-Ts_Mean(Close,20))/Ts_Mean(Close,20)") => {
            let m20 = rolling_mean(close, 20);
            let sub = zip_sub_scalar(close, &m20);
            Ok(zip_div(&sub, &m20))
        }
        // Volatility
        "Ts_Mean(Abs(Delta(Close,1)),14)/Close" => {
            let d = delta(close, 1);
            let a = abs_vals(&d);
            let m14 = rolling_mean_options(&a, 14);
            Ok(zip_div_scalar(&m14, close))
        }
        "2*Ts_Std(Close,20)/Ts_Mean(Close,20)" => {
            let s20 = rolling_std(close, 20);
            let m20 = rolling_mean(close, 20);
            let two_s20: Vec<Option<f64>> = s20.iter().map(|v| v.map(|x| 2.0 * x)).collect();
            Ok(zip_div(&two_s20, &m20))
        }
        "Ts_Std(High/Low-1,10)" => {
            let ratio: Vec<f64> = high
                .iter()
                .zip(low.iter())
                .map(|(h, l)| if *l > 1e-9 { h / l - 1.0 } else { 0.0 })
                .collect();
            Ok(rolling_std(&ratio, 10))
        }
        "Ts_Std(Delta(Close,1)/Delay(Close,1),20)" => {
            let d = delta(close, 1);
            let dl = delay(close, 1);
            let ret: Vec<Option<f64>> = zip_div(&d, &dl);
            let vals: Vec<f64> = ret.iter().map(|v| v.unwrap_or(0.0)).collect();
            Ok(rolling_std(&vals, 20))
        }
        // Volume
        "Volume/Ts_Mean(Volume,5)" => {
            let m5 = rolling_mean(volume, 5);
            Ok(zip_div_scalar(&m5, volume))
        }
        s if s.contains("Delta(Ts_Mean(Volume,5),5)/Ts_Mean(Volume,5)") => {
            let m5 = rolling_mean(volume, 5);
            let vals: Vec<f64> = m5.iter().map(|v| v.unwrap_or(0.0)).collect();
            let d5 = delta(&vals, 5);
            // This expression is Delta(Ts_Mean(Volume,5),5) / Ts_Mean(Volume,5)
            // = (m5[i] - m5[i-5]) / m5[i]
            let mut out = vec![None; volume.len()];
            for i in 5..volume.len() {
                if let (Some(cur), Some(prev)) = (m5[i], m5[i - 5]) {
                    if cur.abs() > 1e-9 {
                        out[i] = Some((cur - prev) / cur);
                    }
                }
            }
            Ok(out)
        }
        // Trend
        "Ts_Mean(Close,5)-Ts_Mean(Close,20)" => {
            let m5 = rolling_mean(close, 5);
            let m20 = rolling_mean(close, 20);
            Ok(zip_sub(&m5, &m20))
        }
        "(Close-Delay(Close,10))/Delay(Close,10)" => {
            let dl = delay(close, 10);
            let sub = zip_sub_scalar(close, &dl);
            Ok(zip_div(&sub, &dl))
        }
        _ => Err(anyhow::anyhow!(
            "unsupported expression: '{}'. Only the 12 built-in factor formulas are supported. \
             To add a new factor, add its match arm in compute.rs.",
            expression
        )),
    }
}

// ── Arithmetic helpers on Option<f64> vectors ───────────────────────────────

fn zip_div(a: &[Option<f64>], b: &[Option<f64>]) -> Vec<Option<f64>> {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| match (x, y) {
            (Some(xv), Some(yv)) if yv.abs() > 1e-12 && xv.is_finite() && yv.is_finite() => {
                Some(xv / yv)
            }
            _ => None,
        })
        .collect()
}

fn zip_sub(a: &[Option<f64>], b: &[Option<f64>]) -> Vec<Option<f64>> {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| match (x, y) {
            (Some(xv), Some(yv)) if xv.is_finite() && yv.is_finite() => Some(xv - yv),
            _ => None,
        })
        .collect()
}

fn zip_div_scalar(nums: &[Option<f64>], denoms: &[f64]) -> Vec<Option<f64>> {
    nums.iter()
        .zip(denoms.iter())
        .map(|(n, d)| {
            if let Some(nv) = n {
                if d.abs() > 1e-12 && nv.is_finite() && d.is_finite() {
                    return Some(nv / d);
                }
            }
            None
        })
        .collect()
}

fn zip_sub_scalar(a_vals: &[f64], b_opts: &[Option<f64>]) -> Vec<Option<f64>> {
    a_vals
        .iter()
        .zip(b_opts.iter())
        .map(|(a, b)| match b {
            Some(bv) if a.is_finite() && bv.is_finite() => Some(a - bv),
            _ => None,
        })
        .collect()
}

fn rolling_mean_options(vals: &[Option<f64>], period: usize) -> Vec<Option<f64>> {
    let flat: Vec<f64> = vals.iter().map(|v| v.unwrap_or(0.0)).collect();
    rolling_mean(&flat, period)
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Evaluate all factor definitions against the candle DataFrame.
///
/// Returns a DataFrame with columns `time` + one column per factor (raw values).
pub fn evaluate_factors_df(df: &DataFrame, factor_defs: &[FactorDef]) -> Result<DataFrame> {
    let close: Vec<f64> = df
        .column("close")?
        .f64()
        .context("close not f64")?
        .into_iter()
        .map(|v| v.unwrap_or(0.0))
        .collect();
    let high: Vec<f64> = df
        .column("high")?
        .f64()
        .context("high not f64")?
        .into_iter()
        .map(|v| v.unwrap_or(0.0))
        .collect();
    let low: Vec<f64> = df
        .column("low")?
        .f64()
        .context("low not f64")?
        .into_iter()
        .map(|v| v.unwrap_or(0.0))
        .collect();
    let volume: Vec<f64> = df
        .column("volume")?
        .f64()
        .context("volume not f64")?
        .into_iter()
        .map(|v| v.unwrap_or(0.0))
        .collect();
    let time_col = df.column("time")?.clone();

    let mut result_cols: Vec<Column> = vec![time_col];

    for f in factor_defs {
        let vals = evaluate_factor_expression(&close, &high, &low, &volume, &f.expression)
            .with_context(|| format!("failed to evaluate factor '{}'", f.name))?;
        result_cols.push(Column::new(f.name.clone().into(), vals));
    }

    Ok(DataFrame::new(result_cols)?)
}

/// High-level convenience: candles → raw factor values DataFrame.
pub fn compute_raw_factors(
    candles: &[Candle],
    factor_defs: &[FactorDef],
) -> Result<DataFrame> {
    let df = candles_to_df(candles);
    evaluate_factors_df(&df, factor_defs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candles(n: usize) -> Vec<Candle> {
        (0..n)
            .map(|i| Candle {
                time: 1_000_000 + i as i64 * 86400,
                open: 10.0 + i as f64 * 0.1,
                high: 10.5 + i as f64 * 0.1,
                low: 9.8 + i as f64 * 0.1,
                close: 10.2 + i as f64 * 0.1,
                volume: 1_000_000.0 + (i as f64).sin() * 500_000.0,
            })
            .collect()
    }

    #[test]
    fn test_candles_to_df() {
        let candles = make_candles(10);
        let df = candles_to_df(&candles);
        assert_eq!(df.height(), 10);
        assert_eq!(df.width(), 6);
    }

    #[test]
    fn test_all_builtin_factors() {
        use crate::services::expression::registry::builtin_factors;
        let candles = make_candles(120);
        let df = compute_raw_factors(&candles, &builtin_factors()).unwrap();
        for f in builtin_factors() {
            assert!(
                df.column(f.name.as_str()).is_ok(),
                "missing column: {}",
                f.name
            );
        }
    }
}