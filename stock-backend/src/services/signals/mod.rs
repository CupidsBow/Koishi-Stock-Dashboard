//! Factor-based signal generation — Stage 4 of the prediction pipeline.
//!
//! Converts `AlphaScore` series into actionable entries:
//!
//! **Factor strategy** (three complementary rules):
//!   1. Quantile threshold — score exceeds historical percentiles
//!   2. Reversal — score crosses zero with momentum
//!   3. Divergence — price and score diverge at extremes
//!
//! **Turtle strategy** (state-machine pyramiding):
//!   Entry / Add / Exit based on alpha z-score thresholds and alpha
//!   improvement relative to entry, with cooldown and stop-loss.

pub mod divergence;
pub mod quantile;
pub mod reversal;
pub mod turtle;

use crate::models::{AlphaScore, Candle, Signal, SignalKind};

pub use turtle::{finalize_actions, generate_turtle_actions, TurtleConfig};

/// Configuration for signal generation thresholds.
pub struct SignalConfig {
    /// Window for percentile reference (e.g. 120)
    pub pct_window: usize,
    /// Window for divergence lookback (e.g. 20)
    pub divergence_window: usize,
    /// Whether to include quantile signals
    pub enable_quantile: bool,
    /// Whether to include reversal signals
    pub enable_reversal: bool,
    /// Whether to include divergence signals
    pub enable_divergence: bool,
}

impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            pct_window: 120,
            divergence_window: 20,
            enable_quantile: true,
            enable_reversal: true,
            enable_divergence: true,
        }
    }
}

/// Generate all factor-based signals from alpha scores and candles.
///
/// Signals are sorted by time before being returned.
pub fn generate_signals(
    scores: &[Option<AlphaScore>],
    candles: &[Candle],
    config: &SignalConfig,
) -> Vec<Signal> {
    let mut all = Vec::new();

    if config.enable_quantile {
        all.extend(quantile::quantile_signals(scores, candles, config.pct_window));
    }
    if config.enable_reversal {
        all.extend(reversal::reversal_signals(scores, candles));
    }
    if config.enable_divergence {
        all.extend(divergence::divergence_signals(
            scores,
            candles,
            config.divergence_window,
        ));
    }

    // Sort by time, dedup same-time signals
    all.sort_by_key(|s| s.time);
    all.dedup_by(|a, b| a.time == b.time && a.reason == b.reason);

    all
}

/// Pair unpaired buy/sell signals and compute PnL percentages for closed
/// positions.  Returns a new `Vec<Signal>` where paired `Buy` signals carry
/// `pnl_pct = Some((exit_price - avg_entry_price) / avg_entry_price * 100)`.
///
/// Logic: walk signals chronologically; each `Buy` opens one position (pushed
/// onto a stack); each `Sell` closes **all** currently-open buys at the sell
/// price.  Remaining unclosed buys keep `pnl_pct = None`.
pub fn pair_signals(signals: &[Signal], candles: &[Candle]) -> Vec<Signal> {
    // Build candle lookup for price at signal time
    let candle_by_time: std::collections::HashMap<i64, f64> =
        candles.iter().map(|c| (c.time, c.close)).collect();

    // Sort by time, then walk chronologically
    let mut sorted: Vec<(usize, &Signal)> =
        signals.iter().enumerate().collect();
    sorted.sort_by_key(|(_, s)| s.time);

    // Track pnl for each signal index
    let n = signals.len();
    let mut pnl_map: Vec<Option<f64>> = vec![None; n];

    // Stack of (original_index, buy_price)
    let mut open: Vec<(usize, f64)> = Vec::new();

    // Walk sorted signals
    for (orig_idx, s) in &sorted {
        match s.kind {
            SignalKind::Buy => {
                open.push((*orig_idx, s.price));
            }
            SignalKind::Sell => {
                let exit_price = candle_by_time
                    .get(&s.time)
                    .copied()
                    .unwrap_or(s.price);
                for (buy_idx, buy_price) in open.drain(..) {
                    let pnl = (exit_price - buy_price) / buy_price * 100.0;
                    pnl_map[buy_idx] = Some(pnl);
                }
            }
        }
    }

    // Build output with pnl_pct filled in where paired
    signals
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut s = s.clone();
            if s.kind == SignalKind::Buy {
                s.pnl_pct = pnl_map[i];
            }
            s
        })
        .collect()
}