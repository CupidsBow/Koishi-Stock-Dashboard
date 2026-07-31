//! Score-reversal signals.
//!
//!   - total crosses from negative to positive AND rising for 3 days → Buy
//!   - total crosses from positive to negative AND falling for 3 days → Sell

use crate::models::{AlphaScore, Candle, Signal, SignalKind};

pub fn reversal_signals(
    scores: &[Option<AlphaScore>],
    candles: &[Candle],
) -> Vec<Signal> {
    let n = scores.len().min(candles.len());
    if n < 5 {
        return Vec::new();
    }

    let mut out = Vec::new();

    for i in 3..n {
        let s0 = &scores[i];
        let s1 = &scores[i - 1];

        let (Some(cur), Some(prev)) = (s0, s1) else { continue };

        // Positive crossover: prev.total <= 0, cur.total > 0
        if prev.total <= 0.0 && cur.total > 0.0 {
            // Check rising for 3 consecutive bars
            let rising = (i.saturating_sub(2)..=i)
                .filter_map(|j| scores.get(j).and_then(|s| s.as_ref()).map(|s| s.total))
                .collect::<Vec<_>>();

            if rising.len() >= 3 && rising.windows(2).all(|w| w[1] > w[0]) {
                let candle = &candles[i];
                out.push(Signal {
                    time: candle.time,
                    kind: SignalKind::Buy,
                    price: candle.close,
                    reason: format!(
                        "Alpha转正+走强 得分:{:.2}→{:.2} [基本面改善]",
                        prev.total, cur.total
                    ),
                    pnl_pct: None,
                });
            }
        }

        // Negative crossover: prev.total >= 0, cur.total < 0
        if prev.total >= 0.0 && cur.total < 0.0 {
            let falling = (i.saturating_sub(2)..=i)
                .filter_map(|j| scores.get(j).and_then(|s| s.as_ref()).map(|s| s.total))
                .collect::<Vec<_>>();

            if falling.len() >= 3 && falling.windows(2).all(|w| w[1] < w[0]) {
                let candle = &candles[i];
                out.push(Signal {
                    time: candle.time,
                    kind: SignalKind::Sell,
                    price: candle.close,
                    reason: format!(
                        "Alpha转负+走弱 得分:{:.2}→{:.2} [基本面恶化]",
                        prev.total, cur.total
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
                close: 10.0 + i as f64 * 0.05,
                volume: 1_000_000.0,
            })
            .collect()
    }

    #[test]
    fn test_buy_crossover_rising() {
        // -0.5, -0.3, -0.1, 0.0 → crossover at idx 4 to +0.2
        let scores = make_scores(&[-0.5, -0.3, -0.1, 0.0, 0.2]);
        let candles = make_candles(5);
        let sigs = reversal_signals(&scores, &candles);
        assert!(!sigs.is_empty());
        assert!(matches!(sigs[0].kind, SignalKind::Buy));
    }

    #[test]
    fn test_no_signal_on_flat() {
        // All zeros — no crossover
        let scores = make_scores(&[0.0, 0.0, 0.0, 0.0, 0.0]);
        let candles = make_candles(5);
        let sigs = reversal_signals(&scores, &candles);
        assert!(sigs.is_empty());
    }
}