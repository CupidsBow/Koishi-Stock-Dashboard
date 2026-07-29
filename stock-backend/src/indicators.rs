use crate::api::Candle;

// ── Bollinger Bands ──────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct BollingerPoint {
    pub time: i64,
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

pub fn bollinger_bands(candles: &[Candle], period: usize) -> Vec<Option<BollingerPoint>> {
    let n = candles.len();
    let mut result = vec![None; n];
    if n < period { return result; }
    for i in (period - 1)..n {
        let slice = &candles[i + 1 - period..=i];
        let sum: f64 = slice.iter().map(|c| c.close).sum();
        let mid = sum / period as f64;
        let var: f64 = slice.iter().map(|c| (c.close - mid).powi(2)).sum::<f64>() / period as f64;
        let std = var.sqrt();
        result[i] = Some(BollingerPoint { time: candles[i].time, upper: mid + 2.0 * std, middle: mid, lower: mid - 2.0 * std });
    }
    result
}

// ── Keltner Channels ─────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct KeltnerPoint {
    pub time: i64,
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

pub fn keltner_channels(candles: &[Candle]) -> Vec<Option<KeltnerPoint>> {
    let n = candles.len();
    let ema_period = 20usize; let atr_period = 10usize; let multiplier = 2.0;
    let mut result = vec![None; n];
    if n < ema_period.max(atr_period) + 1 { return result; }
    let ema_alpha = 2.0 / (ema_period as f64 + 1.0);
    let mut ema_values = vec![0.0f64; n];
    let seed_sum: f64 = candles[..ema_period].iter().map(|c| c.close).sum();
    ema_values[ema_period - 1] = seed_sum / ema_period as f64;
    for i in ema_period..n { ema_values[i] = candles[i].close * ema_alpha + ema_values[i - 1] * (1.0 - ema_alpha); }
    let mut atr_values = vec![0.0f64; n];
    let tr = |i: usize| {
        let h = candles[i].high; let l = candles[i].low; let pc = candles[i - 1].close;
        (h - l).abs().max((h - pc).abs()).max((l - pc).abs())
    };
    let tr_seed_sum: f64 = (1..=atr_period).map(|i| tr(i)).sum();
    atr_values[atr_period] = tr_seed_sum / atr_period as f64;
    let atr_alpha = 1.0 / atr_period as f64;
    for i in atr_period + 1..n { atr_values[i] = tr(i) * atr_alpha + atr_values[i - 1] * (1.0 - atr_alpha); }
    let start = ema_period.max(atr_period + 1);
    for i in start..n {
        let ema = ema_values[i]; let atr = atr_values[i];
        result[i] = Some(KeltnerPoint { time: candles[i].time, upper: ema + multiplier * atr, middle: ema, lower: ema - multiplier * atr });
    }
    result
}

// ── MACD ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct MacdPoint {
    pub time: i64, pub dif: f64, pub dea: f64, pub bar: f64,
}

pub fn macd(candles: &[Candle]) -> Vec<Option<MacdPoint>> {
    let n = candles.len();
    let mut result = vec![None; n];
    if n < 26 + 9 { return result; }
    let a12 = 2.0 / 13.0; let a26 = 2.0 / 27.0; let a9 = 2.0 / 10.0;
    let mut ema12 = vec![0.0f64; n]; let mut ema26 = vec![0.0f64; n];
    ema12[11] = candles[..12].iter().map(|c| c.close).sum::<f64>() / 12.0;
    ema26[25] = candles[..26].iter().map(|c| c.close).sum::<f64>() / 26.0;
    for i in 12..n { ema12[i] = candles[i].close * a12 + ema12[i - 1] * (1.0 - a12); }
    for i in 26..n { ema26[i] = candles[i].close * a26 + ema26[i - 1] * (1.0 - a26); }
    let mut dif = vec![0.0f64; n];
    for i in 25..n { dif[i] = ema12[i] - ema26[i]; }
    let mut dea = vec![0.0f64; n];
    dea[33] = dif[25..=33].iter().sum::<f64>() / 9.0;
    for i in 34..n { dea[i] = dif[i] * a9 + dea[i - 1] * (1.0 - a9); }
    for i in 33..n {
        let bar = 2.0 * (dif[i] - dea[i]);
        result[i] = Some(MacdPoint { time: candles[i].time, dif: dif[i], dea: dea[i], bar });
    }
    result
}

// ── KDJ ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct KdjPoint {
    pub time: i64, pub k: f64, pub d: f64, pub j: f64,
}

pub fn kdj(candles: &[Candle], period: usize) -> Vec<Option<KdjPoint>> {
    let n = candles.len();
    let mut result = vec![None; n];
    if n < period + 1 { return result; }
    let alpha_k = 1.0 / 3.0; let alpha_d = 1.0 / 3.0;
    let mut k_prev = 50.0; let mut d_prev = 50.0;
    for i in (period - 1)..n {
        let slice = &candles[i + 1 - period..=i];
        let high_n = slice.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
        let low_n  = slice.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
        let rsv = if (high_n - low_n).abs() < 1e-9 { 50.0 }
                  else { (candles[i].close - low_n) / (high_n - low_n) * 100.0 };
        let k = alpha_k * rsv + (1.0 - alpha_k) * k_prev;
        let d = alpha_d * k   + (1.0 - alpha_d) * d_prev;
        let j = 3.0 * k - 2.0 * d;
        result[i] = Some(KdjPoint { time: candles[i].time, k, d, j });
        k_prev = k; d_prev = d;
    }
    result
}

// ── Per-pane Signals + Cross-confirmation ────────────────────────────────

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum SignalKind { Buy, Sell }

#[derive(Debug, Clone, serde::Serialize)]
pub struct Signal {
    pub time: i64,
    pub kind: SignalKind,
    pub price: f64,
    pub reason: &'static str,
}

const CONFIRM_WINDOW: usize = 3; // bars within which another pane must echo the signal

fn in_window<F: Fn(usize) -> bool>(i: usize, w: usize, f: F) -> bool {
    (i.saturating_sub(w - 1)..=i).any(|j| f(j))
}

// ── Pane 1: KDJ signals ──────────────────────────────────────────────────

struct KdjSignals { buy_at: Vec<usize>, sell_at: Vec<usize> }

fn kdj_signals(kdj: &[Option<KdjPoint>]) -> KdjSignals {
    let mut buy = vec![]; let mut sell = vec![];
    for i in 1..kdj.len() {
        let Some(p) = &kdj[i - 1] else { continue };
        let Some(c) = &kdj[i] else { continue };
        // Golden cross
        if p.k <= p.d && c.k > c.d { buy.push(i); }
        // Death cross
        if p.k >= p.d && c.k < c.d { sell.push(i); }
        // Oversold bounce (K < 20, curling up)
        if c.k < 20.0 && c.k > p.k && p.d < 25.0  { buy.push(i); }
        // Overbought turn (J > 100, K curling down)
        if c.j > 100.0 && c.k < p.k { sell.push(i); }
    }
    KdjSignals { buy_at: buy, sell_at: sell }
}

// ── Pane 2: MACD signals ─────────────────────────────────────────────────

struct MacdSignals { buy_at: Vec<usize>, sell_at: Vec<usize> }

fn macd_signals(macd: &[Option<MacdPoint>]) -> MacdSignals {
    let mut buy = vec![]; let mut sell = vec![];
    for i in 1..macd.len() {
        let Some(p) = &macd[i - 1] else { continue };
        let Some(c) = &macd[i] else { continue };
        // Golden cross (DIF ↑ DEA)
        if p.dif <= p.dea && c.dif > c.dea && c.dif > -0.02 { buy.push(i); }
        // Death cross (DIF ↓ DEA)
        if p.dif >= p.dea && c.dif < c.dea && c.dif < 0.02 { sell.push(i); }
        // Bar turns positive (momentum shift)
        if p.bar <= 0.0 && c.bar > 0.0 { buy.push(i); }
        // Bar turns negative
        if p.bar >= 0.0 && c.bar < 0.0 { sell.push(i); }
    }
    MacdSignals { buy_at: buy, sell_at: sell }
}

// ── Pane 0: Support/Resistance (KC + BB) signals ─────────────────────────

struct SrSignals { buy_at: Vec<usize>, sell_at: Vec<usize> }

fn sr_signals(
    candles:   &[Candle],
    bollinger: &[Option<BollingerPoint>],
    keltner:   &[Option<KeltnerPoint>],
) -> SrSignals {
    let mut buy = vec![]; let mut sell = vec![];
    let n = candles.len();
    for i in 50..n {
        let Some(bb_i) = &bollinger[i] else { continue };
        let Some(kc_i) = &keltner[i] else { continue };
        let price = candles[i].close;

        // KC lower band support
        if price <= kc_i.lower * 1.02 { buy.push(i); }
        // BB lower band touch + oversold area
        if price <= bb_i.lower * 1.02 { buy.push(i); }
        // KC middle support bounce (uptrend confirmation)
        if price >= kc_i.middle * 0.99 && price <= kc_i.middle * 1.01 { buy.push(i); }

        // KC upper band resistance
        if price >= kc_i.upper * 0.98 { sell.push(i); }
        // BB upper band touch
        if price >= bb_i.upper * 0.98 { sell.push(i); }
    }
    SrSignals { buy_at: buy, sell_at: sell }
}

// ── Merge with cross-confirmation ─────────────────────────────────────────
//
// Rule: a signal from any ONE pane is drawn on the chart IF AND ONLY IF
//       another pane produced a signal in the SAME DIRECTION within the
//       last CONFIRM_WINDOW bars.
//
// The final signal's `time` and `price` come from the *later* of the two
// confirming signals — i.e. the moment when the second pane confirms.

pub fn compute_signals(
    candles:       &[Candle],
    bollinger:     &[Option<BollingerPoint>],
    keltner:       &[Option<KeltnerPoint>],
    macd:          &[Option<MacdPoint>],
    kdj:           &[Option<KdjPoint>],
) -> Vec<Signal> {
    let kdj_sig  = kdj_signals(kdj);
    let macd_sig = macd_signals(macd);
    let sr_sig   = sr_signals(candles, bollinger, keltner);

    let mut signals = Vec::new();

    // ── BUY: find confirming pairs ────────────────────────────────────────

    // KDJ buy confirmed by MACD buy within window
    for &kdj_i in &kdj_sig.buy_at {
        if in_window(kdj_i, CONFIRM_WINDOW, |j| macd_sig.buy_at.contains(&j)) {
            signals.push(Signal {
                time: candles[kdj_i].time, kind: SignalKind::Buy,
                price: candles[kdj_i].close, reason: "KDJ金叉+MACD确认(买)",
            });
        }
    }
    // KDJ buy confirmed by SR buy within window
    for &kdj_i in &kdj_sig.buy_at {
        if in_window(kdj_i, CONFIRM_WINDOW, |j| sr_sig.buy_at.contains(&j)) {
            signals.push(Signal {
                time: candles[kdj_i].time, kind: SignalKind::Buy,
                price: candles[kdj_i].close, reason: "KDJ金叉+支撑位确认(买)",
            });
        }
    }
    // MACD buy confirmed by SR buy within window
    for &macd_i in &macd_sig.buy_at {
        if in_window(macd_i, CONFIRM_WINDOW, |j| sr_sig.buy_at.contains(&j)) {
            signals.push(Signal {
                time: candles[macd_i].time, kind: SignalKind::Buy,
                price: candles[macd_i].close, reason: "MACD金叉+支撑位确认(买)",
            });
        }
    }

    // ── SELL: find confirming pairs ───────────────────────────────────────

    // KDJ sell confirmed by MACD sell within window
    for &kdj_i in &kdj_sig.sell_at {
        if in_window(kdj_i, CONFIRM_WINDOW, |j| macd_sig.sell_at.contains(&j)) {
            signals.push(Signal {
                time: candles[kdj_i].time, kind: SignalKind::Sell,
                price: candles[kdj_i].close, reason: "KDJ死叉+MACD确认(卖)",
            });
        }
    }
    // KDJ sell confirmed by SR sell within window
    for &kdj_i in &kdj_sig.sell_at {
        if in_window(kdj_i, CONFIRM_WINDOW, |j| sr_sig.sell_at.contains(&j)) {
            signals.push(Signal {
                time: candles[kdj_i].time, kind: SignalKind::Sell,
                price: candles[kdj_i].close, reason: "KDJ死叉+压力位确认(卖)",
            });
        }
    }
    // MACD sell confirmed by SR sell within window
    for &macd_i in &macd_sig.sell_at {
        if in_window(macd_i, CONFIRM_WINDOW, |j| sr_sig.sell_at.contains(&j)) {
            signals.push(Signal {
                time: candles[macd_i].time, kind: SignalKind::Sell,
                price: candles[macd_i].close, reason: "MACD死叉+压力位确认(卖)",
            });
        }
    }

    signals
}