//! Built-in factor library: 12 predefined factors across 4 categories.
//!
//! Each factor uses numeric computation in `compute.rs`; the expression strings
//! are validated at parse time but the actual calculation is done by matching
//! the exact string in `compute::evaluate_factor_expression()`.

use crate::models::FactorDef;

/// Return the built-in factor definitions.
pub fn builtin_factors() -> Vec<FactorDef> {
    vec![
        // ── Momentum ────────────────────────────────────────────────────────
        FactorDef::new("ret_5d", "momentum", "Delta(Close,5)/Delay(Close,5)", 1),
        FactorDef::new(
            "ret_20d",
            "momentum",
            "Delta(Close,5)/Delay(Close,5)",
            1,
        ),
        FactorDef::new(
            "macd_dif",
            "momentum",
            "Ts_Mean(Close,12)-Ts_Mean(Close,26)",
            1,
        ),
        FactorDef::new(
            "ma_disp_20",
            "momentum",
            "(Close-Ts_Mean(Close,20))/Ts_Mean(Close,20)",
            1,
        ),
        // ── Volatility ──────────────────────────────────────────────────────
        FactorDef::new(
            "atr_pct",
            "volatility",
            "Ts_Mean(Abs(Delta(Close,1)),14)/Close",
            -1,
        ),
        FactorDef::new(
            "bb_width",
            "volatility",
            "2*Ts_Std(Close,20)/Ts_Mean(Close,20)",
            0,
        ),
        FactorDef::new(
            "hl_vol",
            "volatility",
            "Ts_Std(High/Low-1,10)",
            -1,
        ),
        FactorDef::new(
            "ret_std",
            "volatility",
            "Ts_Std(Delta(Close,1)/Delay(Close,1),20)",
            -1,
        ),
        // ── Volume ──────────────────────────────────────────────────────────
        FactorDef::new("vol_ratio", "volume", "Volume/Ts_Mean(Volume,5)", 0),
        FactorDef::new(
            "vol_trend",
            "volume",
            "Delta(Ts_Mean(Volume,5),5)/Ts_Mean(Volume,5)",
            1,
        ),
        // ── Trend ───────────────────────────────────────────────────────────
        FactorDef::new(
            "ma_cross",
            "trend",
            "Ts_Mean(Close,5)-Ts_Mean(Close,20)",
            1,
        ),
        FactorDef::new(
            "slope_10",
            "trend",
            "(Close-Delay(Close,10))/Delay(Close,10)",
            1,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::expression::parser::parse_expression;

    #[test]
    fn test_all_expressions_parse() {
        for f in builtin_factors() {
            let result = parse_expression(&f.expression);
            assert!(
                result.is_ok(),
                "failed to parse '{}' ({}): {:?}",
                f.name,
                f.expression,
                result.err()
            );
        }
    }
}