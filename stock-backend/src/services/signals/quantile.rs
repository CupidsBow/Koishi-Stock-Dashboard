//! Quantile-threshold signals from alpha scores.
//!
//! Signal logic:
//!   - score > P80(historical) AND score rising     → Strong Buy
//!   - score > P60(historical) AND score > 0        → Buy
//!   - score < P20(historical) AND score falling    → Strong Sell
//!   - score < P40(historical) AND score < 0        → Sell
//!
//! Where "historical" = the last `pct_window` non-None scores.

use crate::models::{AlphaScore, Candle, Signal, SignalKind};

/// Generate quantile-threshold signals from alpha score series.
///
/// Returns `Vec<Signal>` with human-readable reasons.
pub fn quantile_signals(
    scores: &[Option<AlphaScore>],
    candles: &[Candle],
    pct_window: usize,
) -> Vec<Signal> {
    let n = scores.len().min(candles.len());
    if n < pct_window + 5 {
        return Vec::new();
    }

    let mut out = Vec::new();

    // Collect valid total scores and their indices
    let mut history: Vec<(usize, f64)> = Vec::new();

    for i in 0..n {
        let Some(ref score) = scores[i] else { continue };
        history.push((i, score.total));
    }

    if history.len() < pct_window {
        return out;
    }

    for &(i, total) in &history {
        // Find historical context: scores before this index
        let past: Vec<f64> = history
            .iter()
            .filter(|&&(j, _)| j < i)
            .rev()
            .take(pct_window)
            .map(|(_, v)| *v)
            .collect();

        if past.len() < pct_window / 2 {
            continue;
        }

        let (p80, p60, p40, p20) = compute_quantiles(past);
        let rising = is_rising(scores, i, 5);
        let falling = is_falling(scores, i, 5);

        // Find dominant category for the reason string
        let dominant = dominant_category(scores, i);

        let candle = &candles[i];

        if total > p80 && rising {
            out.push(Signal {
                time: candle.time,
                kind: SignalKind::Buy,
                price: candle.close,
                reason: format!(
                    "Alpha得分:{:.2}>P80({:.2}) 强买 [{}]",
                    total, p80, dominant
                ),
                pnl_pct: None,
            });
        } else if total > p60 && total > 0.0 {
            out.push(Signal {
                time: candle.time,
                kind: SignalKind::Buy,
                price: candle.close,
                reason: format!(
                    "Alpha得分:{:.2}>P60({:.2}) 偏多 [{}]",
                    total, p60, dominant
                ),
                pnl_pct: None,
            });
        } else if total < p20 && falling {
            out.push(Signal {
                time: candle.time,
                kind: SignalKind::Sell,
                price: candle.close,
                reason: format!(
                    "Alpha得分:{:.2}<P20({:.2}) 强卖 [{}]",
                    total, p20, dominant
                ),
                pnl_pct: None,
            });
        } else if total < p40 && total < 0.0 {
            out.push(Signal {
                time: candle.time,
                kind: SignalKind::Sell,
                price: candle.close,
                reason: format!(
                    "Alpha得分:{:.2}<P40({:.2}) 偏空 [{}]",
                    total, p40, dominant
                ),
                pnl_pct: None,
            });
        }
    }

    out
}

/// Compute P80, P60, P40, P20 from a sorted list.
fn compute_quantiles(mut vals: Vec<f64>) -> (f64, f64, f64, f64) {
    vals.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    let n = vals.len();
    let idx = |pct: f64| ((n - 1) as f64 * pct) as usize;
    (
        vals[idx(0.80)],
        vals[idx(0.60)],
        vals[idx(0.40)],
        vals[idx(0.20)],
    )
}

/// Check if the total score has been trending up over the last `window` bars.
fn is_rising(scores: &[Option<AlphaScore>], i: usize, window: usize) -> bool {
    if i < window {
        return false;
    }
    let start = i.saturating_sub(window);
    let vals: Vec<f64> = (start..=i)
        .filter_map(|j| scores.get(j).and_then(|s| s.as_ref()).map(|s| s.total))
        .collect();
    if vals.len() < 3 {
        return false;
    }
    vals.last().unwrap() > vals.first().unwrap()
}

fn is_falling(scores: &[Option<AlphaScore>], i: usize, window: usize) -> bool {
    if i < window {
        return false;
    }
    let start = i.saturating_sub(window);
    let vals: Vec<f64> = (start..=i)
        .filter_map(|j| scores.get(j).and_then(|s| s.as_ref()).map(|s| s.total))
        .collect();
    if vals.len() < 3 {
        return false;
    }
    vals.last().unwrap() < vals.first().unwrap()
}

/// Find the category with the largest absolute contribution at index `i`.
fn dominant_category(scores: &[Option<AlphaScore>], i: usize) -> String {
    let Some(ref s) = scores[i] else {
        return "未知".into()
    };
    let cats = [
        ("动量↑", s.momentum),
        ("波动", s.volatility),
        ("量价", s.volume),
        ("趋势", s.trend),
    ];
    let best = cats
        .iter()
        .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
        .unwrap();
    if best.1 > 0.0 {
        best.0.to_string()
    } else if best.1 < 0.0 {
        best.0.replace('↑', "↓")
    } else {
        best.0.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_scores(totals: &[f64]) -> Vec<Option<AlphaScore>> {
        totals
            .iter()
            .enumerate()
            .map(|(i, &t)| {
                Some(AlphaScore {
                    time: 1_000_000 + i as i64 * 86400,
                    momentum: 0.0,
                    volatility: 0.0,
                    volume: 0.0,
                    trend: 0.0,
                    total: t,
                })
            })
            .collect()
    }

    fn make_candles(n: usize) -> Vec<Candle> {
        (0..n)
            .map(|i| Candle {
                time: 1_000_000 + i as i64 * 86400,
                open: 10.0,
                high: 10.5,
                low: 9.8,
                close: 10.0 + i as f64 * 0.1,
                volume: 1_000_000.0,
            })
            .collect()
    }

    #[test]
    fn test_no_signal_when_low_data() {
        let scores = make_scores(&[0.5, 0.6, 0.7]);
        let candles = make_candles(3);
        let sigs = quantile_signals(&scores, &candles, 120);
        assert!(sigs.is_empty());
    }
}