// ── Turtle Position Management Strategy ──────────────────────────────────────
//!
//! Classic pyramiding-based position sizing applied to the Alpha Score
//! (rather than raw price changes).  The algorithm walks through the
//! Alpha Score series chronologically, maintaining a simple state machine.
//!
//! For each bar (once enough history exists to compute z-scores):
//!
//!   • Not in a position and alpha_z > entry_threshold → Entry (1 unit)
//!   • In a position and alpha improves by add_step × σ … → Add (+1 unit)
//!   • In a position and alpha drops below entry − stop_loss × σ … → Exit
//!
//! A configurable cooldown prevents consecutive same-direction actions.

use crate::models::{ActionKind, ActionPoint, AlphaScore, Candle};

// ── Configuration ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TurtleConfig {
    /// Entry trigger: alpha z-score must exceed this threshold.
    /// Default 1.5 — roughly top 7% of alpha values.
    pub entry_z_threshold: f64,
    /// How much alpha must improve (in multiples of its rolling σ)
    /// before another pyramid layer is added.
    pub add_step_sigma: f64,
    /// Stop-loss: if alpha drops below (entry_alpha − stop_loss × σ)
    /// the entire position is closed.
    pub stop_loss_sigma: f64,
    /// Maximum number of pyramid layers (each layer = unit_size_pct of capital).
    pub max_units: u32,
    /// Capital allocated per layer (0–1).  Default 0.25 → 4 layers = 100%.
    pub unit_size_pct: f64,
    /// Minimum number of bars between consecutive actions.
    pub cooldown_bars: u32,
    /// Lookback window for computing rolling z-score and standard deviation
    /// of the alpha score series.
    pub alpha_lookback: usize,
}

impl Default for TurtleConfig {
    fn default() -> Self {
        Self {
            entry_z_threshold: 1.5,
            add_step_sigma: 0.5,
            stop_loss_sigma: 2.0,
            max_units: 4,
            unit_size_pct: 0.25,
            cooldown_bars: 3,
            alpha_lookback: 60,
        }
    }
}

// ── Rolling Statistics ─────────────────────────────────────────────────────

/// Compute a rolling z-score series for the Alpha Score `total` field.
///
/// For each bar `i >= lookback`, the z-score is:
///
/// ```text
/// z[i] = (total[i] − μ[window]) / σ[window]
/// ```
///
/// where μ and σ are computed over `[i−lookback+1, i]`.
fn compute_rolling_zscore(
    scores: &[Option<AlphaScore>],
    lookback: usize,
) -> Vec<Option<f64>> {
    let n = scores.len();
    let mut out = vec![None; n];
    if n < lookback {
        return out;
    }

    for i in lookback..n {
        let window_start = i.saturating_sub(lookback - 1);
        let window: Vec<f64> = (window_start..=i)
            .filter_map(|j| scores.get(j).and_then(|s| s.as_ref()).map(|s| s.total))
            .filter(|v| v.is_finite())
            .collect();

        let w_len = window.len();
        if w_len < lookback / 3 {
            continue; // not enough valid data
        }

        let mean = window.iter().sum::<f64>() / w_len as f64;
        let var = window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / w_len as f64;
        let sigma = var.sqrt().max(1e-12);

        if let Some(score) = scores.get(i).and_then(|s| s.as_ref()) {
            if score.total.is_finite() {
                out[i] = Some((score.total - mean) / sigma);
            }
        }
    }

    out
}

/// Compute a rolling standard deviation series for the Alpha Score `total` field.
fn compute_rolling_std(
    scores: &[Option<AlphaScore>],
    lookback: usize,
) -> Vec<Option<f64>> {
    let n = scores.len();
    let mut out = vec![None; n];
    if n < lookback {
        return out;
    }

    for i in lookback..n {
        let window_start = i.saturating_sub(lookback - 1);
        let window: Vec<f64> = (window_start..=i)
            .filter_map(|j| scores.get(j).and_then(|s| s.as_ref()).map(|s| s.total))
            .filter(|v| v.is_finite())
            .collect();

        let w_len = window.len();
        if w_len < lookback / 3 {
            continue;
        }

        let mean = window.iter().sum::<f64>() / w_len as f64;
        let var = window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / w_len as f64;
        out[i] = Some(var.sqrt().max(1e-12));
    }

    out
}

// ── State Machine ──────────────────────────────────────────────────────────

struct PositionState {
    in_position: bool,
    /// Alpha Score (total) at the original entry bar.
    entry_alpha: f64,
    /// Weighted average entry price across all layers.
    avg_entry_price: f64,
    /// How many pyramid layers are currently held.
    current_units: u32,
    /// Alpha Score at the most recent add (or entry if no adds yet).
    last_add_alpha: f64,
    /// Remaining cooldown bars (decremented each bar).
    cooldown: u32,
}

impl PositionState {
    fn new() -> Self {
        Self {
            in_position: false,
            entry_alpha: 0.0,
            avg_entry_price: 0.0,
            current_units: 0,
            last_add_alpha: 0.0,
            cooldown: 0,
        }
    }

    fn reset(&mut self) {
        self.in_position = false;
        self.entry_alpha = 0.0;
        self.avg_entry_price = 0.0;
        self.current_units = 0;
        self.last_add_alpha = 0.0;
        self.cooldown = 0;
    }
}

// ── Main Entry Point ───────────────────────────────────────────────────────

/// Generate Turtle trading action points from an Alpha Score series.
///
/// # Arguments
/// * `scores` — Alpha score series (one per bar).
/// * `candles` — OHLCV data, aligned with `scores`.
/// * `config` — Turtle strategy parameters (see [`TurtleConfig`]).
///
/// # Returns
/// A chronological list of `ActionPoint`s with PnL populated on Exit actions.
pub fn generate_turtle_actions(
    scores: &[Option<AlphaScore>],
    candles: &[Candle],
    config: &TurtleConfig,
) -> Vec<ActionPoint> {
    let n = scores.len().min(candles.len());
    if n < config.alpha_lookback {
        return Vec::new();
    }

    // Pre-compute rolling statistics
    let z_scores = compute_rolling_zscore(scores, config.alpha_lookback);
    let rolling_std = compute_rolling_std(scores, config.alpha_lookback);

    let mut state = PositionState::new();
    let mut actions: Vec<ActionPoint> = Vec::new();

    // Only process bars with enough history for the rolling stats.
    for i in config.alpha_lookback..n {
        // Decrement cooldown
        if state.cooldown > 0 {
            state.cooldown -= 1;
            // Still skip decision-making during cooldown
            // (but we still re-evaluate stop-loss even during cooldown).
        }

        let Some(ref score) = scores[i] else {
            continue;
        };
        let cur_alpha = score.total;
        if !cur_alpha.is_finite() {
            continue;
        }

        let z = z_scores[i].unwrap_or(0.0);
        let sigma = rolling_std[i].unwrap_or(1e-6);
        let candle = &candles[i];

        // ── Exit check (always evaluated, even during cooldown) ──────────
        if state.in_position {
            let stop_level = state.entry_alpha - config.stop_loss_sigma * sigma;
            if cur_alpha < stop_level {
                let pnl = (candle.close - state.avg_entry_price) / state.avg_entry_price * 100.0;
                actions.push(ActionPoint {
                    time: candle.time,
                    action: ActionKind::Exit,
                    price: candle.close,
                    alpha_score: cur_alpha,
                    alpha_z: z,
                    alpha_std: sigma,
                    position_pct: 0.0,
                    current_units: 0,
                    max_units: config.max_units,
                    reason: format!(
                        "止损平仓:Alpha{:.3}<止损线{:.3}(建仓{:.3}-{}σ) PnL{:+.2}%",
                        cur_alpha, stop_level, state.entry_alpha,
                        config.stop_loss_sigma, pnl
                    ),
                    pnl_pct: Some(pnl),
                });
                state.reset();
                state.cooldown = config.cooldown_bars;
                continue;
            }
        }

        // ── Decision-making (only when cooldown expired) ─────────────────
        if state.cooldown > 0 {
            continue;
        }

        if !state.in_position {
            // ── Entry ────────────────────────────────────────────────────
            if z > config.entry_z_threshold {
                let position_pct = config.unit_size_pct * 100.0;
                state.in_position = true;
                state.entry_alpha = cur_alpha;
                state.avg_entry_price = candle.close;
                state.current_units = 1;
                state.last_add_alpha = cur_alpha;
                state.cooldown = config.cooldown_bars;

                actions.push(ActionPoint {
                    time: candle.time,
                    action: ActionKind::Entry,
                    price: candle.close,
                    alpha_score: cur_alpha,
                    alpha_z: z,
                    alpha_std: sigma,
                    position_pct,
                    current_units: 1,
                    max_units: config.max_units,
                    reason: format!(
                        "建仓:Alpha_z={:.2}>阈值{:.1} 仓位{:.0}%",
                        z, config.entry_z_threshold, position_pct
                    ),
                    pnl_pct: None,
                });
            }
        } else {
            // ── Add (Pyramiding) ─────────────────────────────────────────
            let add_level = state.last_add_alpha + config.add_step_sigma * sigma;
            if cur_alpha > add_level && state.current_units < config.max_units {
                // Update average entry price (weighted by layers)
                let new_units = state.current_units + 1;
                let old_weight = state.current_units as f64;
                let new_weight = 1.0;
                state.avg_entry_price =
                    (state.avg_entry_price * old_weight + candle.close * new_weight)
                        / new_units as f64;
                state.current_units = new_units;
                state.last_add_alpha = cur_alpha;
                state.cooldown = config.cooldown_bars;

                let position_pct = state.current_units as f64 * config.unit_size_pct * 100.0;

                actions.push(ActionPoint {
                    time: candle.time,
                    action: ActionKind::Add,
                    price: candle.close,
                    alpha_score: cur_alpha,
                    alpha_z: z,
                    alpha_std: sigma,
                    position_pct,
                    current_units: state.current_units,
                    max_units: config.max_units,
                    reason: format!(
                        "加仓:Alpha{:.3}>加仓线{:.3}(前高{:.3}+{}σ) 仓位{:.0}% 均价{:.2}",
                        cur_alpha, add_level, state.last_add_alpha - config.add_step_sigma * sigma,
                        config.add_step_sigma, position_pct, state.avg_entry_price
                    ),
                    pnl_pct: None,
                });
            }
        }
    }

    // ── End-of-series marker (unclosed position) ─────────────────────────
    if state.in_position {
        // The last bar in the series — mark the open position.
        // We don't add an Exit; the frontend can infer this as "still open".
        // We do add a final Entry/Add if there was an active position at end,
        // but the existing Entry/Add actions already represent it.
        //
        // For clarity, tag the last action (which is the most recent Entry/Add)
        // with an open-position note.  We do this by appending a virtual
        // "Comment" note — but since ActionKind only has Entry/Add/Exit,
        // we just leave it as-is.  The frontend can detect that the last
        // action's pnl_pct is None and the position_pct > 0.
        let _ = state; // suppress unused warning
    }

    actions
}

// ── Post-processing ────────────────────────────────────────────────────────

/// Finalize turtle actions: compute PnL on Exit and mark unclosed positions.
///
/// This is a thin wrapper — PnL is already computed in `generate_turtle_actions`
/// at Exit time.  This function exists for symmetry with `pair_signals` and
/// may be extended later (e.g., adding trailing-stop-only Exits).
pub fn finalize_actions(actions: &[ActionPoint]) -> Vec<ActionPoint> {
    actions.to_vec()
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an AlphaScore series from a slice of total values.
    /// time starts at 1_000_000 and increments by 86400 (1 day).
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
                open: 10.0 + i as f64 * 0.1,
                high: 10.5 + i as f64 * 0.1,
                low: 9.8 + i as f64 * 0.1,
                close: 10.2 + i as f64 * 0.1,
                volume: 1_000_000.0,
            })
            .collect()
    }

    // ── Data-availability tests ────────────────────────────────────────

    #[test]
    fn test_not_enough_data() {
        let scores = make_scores(&[0.5, 0.6, 0.7]);
        let candles = make_candles(3);
        let config = TurtleConfig {
            alpha_lookback: 10,
            ..Default::default()
        };
        let actions = generate_turtle_actions(&scores, &candles, &config);
        assert!(actions.is_empty(), "should return empty with insufficient data");
    }

    // ── Entry test ─────────────────────────────────────────────────────

    #[test]
    fn test_entry_on_high_zscore() {
        // Build 80 bars: first 60 flat around 0 (establish baseline),
        // then a sudden jump to high values (z-score will be high).
        let mut totals = vec![0.0; 65];
        totals.extend(vec![2.0, 2.5, 3.0, 2.8, 3.2, 3.5, 3.8]);
        let scores = make_scores(&totals);
        let candles = make_candles(totals.len());
        let config = TurtleConfig {
            alpha_lookback: 60,
            entry_z_threshold: 1.5,
            cooldown_bars: 1,
            ..Default::default()
        };
        let actions = generate_turtle_actions(&scores, &candles, &config);
        // Should have at least one Entry
        let entries: Vec<_> = actions.iter().filter(|a| a.action == ActionKind::Entry).collect();
        assert!(!entries.is_empty(), "expected at least one Entry action");
    }

    // ── Stop-loss test ─────────────────────────────────────────────────

    #[test]
    fn test_stop_loss_triggers_exit() {
        // Baseline + entry + sharp drop
        let mut totals = vec![0.0; 65];
        // High values — trigger entry
        totals.extend(vec![3.0, 3.2, 3.5]);
        // Then crash below entry_alpha − 2σ
        totals.extend(vec![-1.0, -1.5, -2.0, -2.5]);
        let scores = make_scores(&totals);
        let candles = make_candles(totals.len());
        let config = TurtleConfig {
            alpha_lookback: 60,
            entry_z_threshold: 1.5,
            stop_loss_sigma: 2.0,
            cooldown_bars: 1,
            ..Default::default()
        };
        let actions = generate_turtle_actions(&scores, &candles, &config);
        let exits: Vec<_> = actions.iter().filter(|a| a.action == ActionKind::Exit).collect();
        assert!(!exits.is_empty(), "expected at least one Exit (stop loss)");
        let exit = exits[0];
        assert!(exit.pnl_pct.is_some(), "exit should carry pnl_pct");
    }

    // ── Cooldown test ──────────────────────────────────────────────────

    #[test]
    fn test_cooldown_prevents_rapid_actions() {
        // Sustained high values — should only produce actions spaced apart
        let mut totals = vec![0.0; 65];
        totals.extend(vec![3.0, 3.3, 3.6, 3.9, 4.2, 4.5, 4.8, 5.1]);
        let scores = make_scores(&totals);
        let candles = make_candles(totals.len());
        let config = TurtleConfig {
            alpha_lookback: 60,
            entry_z_threshold: 1.5,
            add_step_sigma: 0.5,
            cooldown_bars: 3,  // 3-bar cooldown
            max_units: 4,
            ..Default::default()
        };
        let actions = generate_turtle_actions(&scores, &candles, &config);
        // Each action's time should differ by at least cooldown_bars * 86400
        for w in actions.windows(2) {
            let dt = w[1].time - w[0].time;
            assert!(
                dt >= config.cooldown_bars as i64 * 86400,
                "actions too close: dt={} seconds, expected >= {}",
                dt,
                config.cooldown_bars as i64 * 86400
            );
        }
    }

    // ── Full cycle test ────────────────────────────────────────────────

    #[test]
    fn test_full_cycle_entry_add_exit() {
        let config = TurtleConfig {
            alpha_lookback: 60,
            entry_z_threshold: 1.5,
            add_step_sigma: 0.5,
            stop_loss_sigma: 2.0,
            max_units: 4,
            cooldown_bars: 1, // fast cooldown for test
            ..Default::default()
        };

        // 60 bars of baseline, then ramp up for entry + adds, then crash for exit
        let mut totals = vec![0.0; 65];
        // Warm entry
        totals.extend(vec![2.0, 2.5, 3.0]);
        // Keep going up → adds
        totals.extend(vec![3.5, 4.0, 4.5, 5.0, 5.5, 6.0]);
        // Crash below stop level
        totals.extend(vec![-2.0, -2.5, -3.0]);

        let scores = make_scores(&totals);
        let candles = make_candles(totals.len());
        let actions = generate_turtle_actions(&scores, &candles, &config);

        let entries: Vec<_> = actions.iter().filter(|a| a.action == ActionKind::Entry).collect();
        let adds: Vec<_> = actions.iter().filter(|a| a.action == ActionKind::Add).collect();
        let exits: Vec<_> = actions.iter().filter(|a| a.action == ActionKind::Exit).collect();

        assert_eq!(entries.len(), 1, "expected exactly 1 entry");
        assert!(!adds.is_empty(), "expected at least 1 add");
        assert_eq!(exits.len(), 1, "expected exactly 1 exit");

        // Position size should increase with each add
        for add in &adds {
            assert!(add.position_pct > 25.0, "position should grow with adds");
        }
        assert_eq!(exits[0].position_pct, 0.0, "exit should clear position");
    }

    // ── Max units clamp ────────────────────────────────────────────────

    #[test]
    fn test_max_units_clamp() {
        let mut totals = vec![0.0; 65];
        // Rapid ramp to test that we don't exceed max_units
        totals.extend(vec![3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);
        let scores = make_scores(&totals);
        let candles = make_candles(totals.len());
        let config = TurtleConfig {
            alpha_lookback: 60,
            entry_z_threshold: 1.5,
            add_step_sigma: 0.3,
            max_units: 3,
            cooldown_bars: 1,
            ..Default::default()
        };
        let actions = generate_turtle_actions(&scores, &candles, &config);
        for a in &actions {
            assert!(
                a.current_units <= config.max_units,
                "current_units {} exceeded max_units {}",
                a.current_units,
                config.max_units
            );
        }
    }
}