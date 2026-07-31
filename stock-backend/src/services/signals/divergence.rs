//! Price-factor divergence signals.
//!
//!   - Price makes a new 20-day high but alpha score does NOT → Sell (top divergence)
//!   - Price makes a new 20-day low but alpha score does NOT → Buy (bottom divergence)
//!
//! Requires the score to be trending opposite to price for at least 5 bars.

use crate::models::{AlphaScore, Candle, Signal, SignalKind};

pub fn divergence_signals(
    scores: &[Option<AlphaScore>],
    candles: &[Candle],
    lookback: usize,
) -> Vec<Signal> {
    let n = scores.len().min(candles.len());
    if n < lookback + 5 {
        return Vec::new();
    }

    let mut out = Vec::new();

    for i in lookback..n {
        let Some(ref score) = scores[i] else { continue };

        let start = i.saturating_sub(lookback);

        // Price 20-day high check
        let price_range: Vec<f64> = (start..i).map(|j| candles[j].close).collect();
        let score_range: Vec<f64> = (start..i)
            .filter_map(|j| scores.get(j).and_then(|s| s.as_ref()).map(|s| s.total))
            .collect();

        if price_range.is_empty() || score_range.is_empty() {
            continue;
        }

        let price_max = price_range.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let price_min = price_range.iter().cloned().fold(f64::INFINITY, f64::min);
        let score_max = score_range.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let score_min = score_range.iter().cloned().fold(f64::INFINITY, f64::min);

        // Top divergence: price at new high, score NOT at new high, AND score falling
        if candles[i].close >= price_max * 0.995 && score.total < score_max * 0.9 {
            // Score must be trending down
            let score_5: Vec<f64> = (i.saturating_sub(4)..=i)
                .filter_map(|j| scores.get(j).and_then(|s| s.as_ref()).map(|s| s.total))
                .collect();
            if score_5.len() >= 5 && score_5.last().unwrap() < score_5.first().unwrap() {
                out.push(Signal {
                    time: candles[i].time,
                    kind: SignalKind::Sell,
                    price: candles[i].close,
                    reason: format!(
                        "顶背离:价格新高{:.2}但Alpha未跟随({:.2}<{:.2})",
                        candles[i].close, score.total, score_max
                    ),
                    pnl_pct: None,
                });
            }
        }

        // Bottom divergence: price at new low, score NOT at new low, AND score rising
        if candles[i].close <= price_min * 1.005 && score.total > score_min * 1.1 {
            let score_5: Vec<f64> = (i.saturating_sub(4)..=i)
                .filter_map(|j| scores.get(j).and_then(|s| s.as_ref()).map(|s| s.total))
                .collect();
            if score_5.len() >= 5 && score_5.last().unwrap() > score_5.first().unwrap() {
                out.push(Signal {
                    time: candles[i].time,
                    kind: SignalKind::Buy,
                    price: candles[i].close,
                    reason: format!(
                        "底背离:价格新低{:.2}但Alpha未跟随({:.2}>{:.2})",
                        candles[i].close, score.total, score_min
                    ),
                    pnl_pct: None,
                });
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_signal_without_enough_data() {
        let scores: Vec<Option<AlphaScore>> = (0..5)
            .map(|i| {
                Some(AlphaScore {
                    time: 1_000_000 + i as i64 * 86400,
                    momentum: 0.0,
                    volatility: 0.0,
                    volume: 0.0,
                    trend: 0.0,
                    total: 0.0,
                })
            })
            .collect();
        let candles: Vec<Candle> = (0..5)
            .map(|i| Candle {
                time: 1_000_000 + i as i64 * 86400,
                open: 10.0,
                high: 10.5,
                low: 9.8,
                close: 10.0,
                volume: 1.0,
            })
            .collect();
        let sigs = divergence_signals(&scores, &candles, 20);
        assert!(sigs.is_empty());
    }
}