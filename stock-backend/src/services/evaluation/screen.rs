//! Factor screening: from rolling IC → FactorEval with validity check.
//!
//! Each factor is evaluated based on its IC series:
//!   - ic_mean = mean of valid IC values over the evaluation window
//!   - ic_std  = std of valid IC values
//!   - ir = ic_mean / max(ic_std, 1e-9)
//!   - is_valid = |ir| > 0.3 || |ic_mean| > 0.02
//!   - weight = |ic_mean| if valid, else 0.0
//!
//! If ic_mean < 0 (negative correlation with forward return), the factor
//! direction is automatically reversed (weight = |ic_mean|) — this is the
//! "auto-detect" path for factors with `direction=0`.

use std::collections::HashMap;

use crate::models::FactorEval;

/// Evaluate all factors from their rolling IC series.
///
/// `ic_map`: factor_name → ic_series (Vec<Option<f64>>)
/// `eval_count`: number of most recent valid IC values to use (e.g. 60)
pub fn evaluate_all(
    ic_map: &HashMap<String, Vec<Option<f64>>>,
    eval_count: usize,
) -> Vec<FactorEval> {
    let mut result = Vec::with_capacity(ic_map.len());

    for (name, ic_series) in ic_map {
        // Take the last `eval_count` valid (non-None) IC values
        let valid_ics: Vec<f64> = ic_series
            .iter()
            .rev()
            .filter_map(|v| *v)
            .take(eval_count)
            .collect();

        if valid_ics.len() < eval_count / 3 {
            // Too few data points — treat as invalid
            result.push(FactorEval {
                name: name.clone(),
                ic_mean: 0.0,
                ir: 0.0,
                weight: 0.0,
                is_valid: false,
                ic_series: ic_series.clone(),
            });
            continue;
        }

        let n = valid_ics.len() as f64;
        let ic_mean = valid_ics.iter().sum::<f64>() / n;
        let ic_std = {
            let var = valid_ics.iter().map(|v| (v - ic_mean).powi(2)).sum::<f64>() / n;
            var.sqrt()
        };

        let ir = if ic_std > 1e-9 {
            ic_mean / ic_std
        } else {
            0.0
        };

        let is_valid = ir.abs() > 0.3 || ic_mean.abs() > 0.02;

        let weight = if is_valid { ic_mean.abs() } else { 0.0 };

        result.push(FactorEval {
            name: name.clone(),
            ic_mean,
            ir,
            weight,
            is_valid,
            ic_series: ic_series.clone(),
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_factor() {
        let mut map = HashMap::new();
        // Simulate a factor with consistently positive IC (~0.05)
        let series: Vec<Option<f64>> = (0..100)
            .map(|i| {
                if i < 10 {
                    None
                } else {
                    Some(0.05 + (i as f64 * 0.001).sin() * 0.02)
                }
            })
            .collect();
        map.insert("good_factor".into(), series);

        let evals = evaluate_all(&map, 60);
        let eval = evals.first().unwrap();
        assert!(eval.is_valid, "good_factor should be valid");
        assert!(eval.weight > 0.0, "good_factor should have positive weight");
    }

    #[test]
    fn test_noise_factor() {
        let mut map = HashMap::new();
        let series: Vec<Option<f64>> = (0..100)
            .map(|i| {
                if i < 10 {
                    None
                } else {
                    // Mean ~0, random noise
                    Some((i as f64).sin() * 0.01)
                }
            })
            .collect();
        map.insert("noise_factor".into(), series);

        let evals = evaluate_all(&map, 60);
        let eval = evals.first().unwrap();
        assert!(!eval.is_valid, "noise_factor should NOT be valid");
        assert_eq!(eval.weight, 0.0, "noise_factor weight should be 0");
    }
}