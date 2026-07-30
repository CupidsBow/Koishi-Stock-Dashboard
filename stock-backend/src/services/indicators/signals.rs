use crate::models::{BollingerPoint, Candle, KdjPoint, KeltnerPoint, MacdPoint, Signal, SignalKind};

const CONFIRM_WINDOW: usize = 3;

fn in_window<F: Fn(usize) -> bool>(i: usize, w: usize, f: F) -> bool {
    (i.saturating_sub(w - 1)..=i).any(|j| f(j))
}

struct KdjSignals {
    kd_buy: Vec<usize>,
    kd_sell: Vec<usize>,
    j_buy: Vec<usize>,
    j_sell: Vec<usize>,
}

fn kdj_signals(kdj: &[Option<KdjPoint>]) -> KdjSignals {
    let (mut kb, mut ks, mut jb, mut js) = (vec![], vec![], vec![], vec![]);
    for i in 1..kdj.len() {
        let (Some(p), Some(c)) = (&kdj[i - 1], &kdj[i]) else {
            continue;
        };
        if p.k <= p.d && c.k > c.d {
            kb.push(i);
        }
        if p.k >= p.d && c.k < c.d {
            ks.push(i);
        }
        if c.k < 20.0 && c.k > p.k {
            kb.push(i);
        }
        if c.j > 100.0 && c.k < p.k {
            ks.push(i);
        }
        if p.j < 0.0 && c.j > p.j {
            jb.push(i);
        }
        if p.j < -5.0 && c.j > p.j {
            jb.push(i);
        }
        if p.j > 100.0 && c.j < p.j {
            js.push(i);
        }
        if p.j > 105.0 && c.j < p.j {
            js.push(i);
        }
    }
    KdjSignals {
        kd_buy: kb,
        kd_sell: ks,
        j_buy: jb,
        j_sell: js,
    }
}

struct MacdSignals {
    buy_at: Vec<usize>,
    sell_at: Vec<usize>,
}

fn macd_signals(macd: &[Option<MacdPoint>]) -> MacdSignals {
    let (mut b, mut s) = (vec![], vec![]);
    for i in 1..macd.len() {
        let (Some(p), Some(c)) = (&macd[i - 1], &macd[i]) else {
            continue;
        };
        if p.dif <= p.dea && c.dif > c.dea && c.dif > -0.02 {
            b.push(i);
        }
        if p.dif >= p.dea && c.dif < c.dea && c.dif < 0.02 {
            s.push(i);
        }
        if p.bar <= 0.0 && c.bar > 0.0 {
            b.push(i);
        }
        if p.bar >= 0.0 && c.bar < 0.0 {
            s.push(i);
        }
    }
    MacdSignals {
        buy_at: b,
        sell_at: s,
    }
}

struct SrSignals {
    buy_at: Vec<usize>,
    sell_at: Vec<usize>,
}

fn sr_signals(
    candles: &[Candle],
    bb: &[Option<BollingerPoint>],
    kc: &[Option<KeltnerPoint>],
) -> SrSignals {
    let (mut b, mut s) = (vec![], vec![]);
    for i in 50..candles.len() {
        let (Some(bi), Some(ki)) = (&bb[i], &kc[i]) else {
            continue;
        };
        let p = candles[i].close;
        if p <= ki.lower * 1.02 {
            b.push(i);
        }
        if p <= bi.lower * 1.02 {
            b.push(i);
        }
        if p >= ki.middle * 0.99 && p <= ki.middle * 1.01 {
            b.push(i);
        }
        if p >= ki.upper * 0.98 {
            s.push(i);
        }
        if p >= bi.upper * 0.98 {
            s.push(i);
        }
    }
    SrSignals {
        buy_at: b,
        sell_at: s,
    }
}

// ── BB Squeeze + Volume Breakout Signals ──────────────────────────────────

fn vol_ma(candles: &[Candle], i: usize, period: usize) -> f64 {
    if i < period - 1 {
        return 0.0;
    }
    candles[i + 1 - period..=i]
        .iter()
        .map(|c| c.volume)
        .sum::<f64>()
        / period as f64
}

/// 20-day volume 75th percentile at index i
fn vol_pct75(candles: &[Candle], i: usize) -> f64 {
    if i < 19 {
        return 0.0;
    }
    let mut vols: Vec<f64> = candles[i - 19..=i].iter().map(|c| c.volume).collect();
    vols.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    // 75th percentile for 20 elements = index 14 (0-based)
    vols[14]
}

fn bb_squeeze_signals(
    candles: &[Candle],
    bb: &[Option<BollingerPoint>],
    kc: &[Option<KeltnerPoint>],
    macd: &[Option<MacdPoint>],
    kdj: &[Option<KdjPoint>],
) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    let n = candles.len();
    let mut buys: Vec<usize> = Vec::new();
    let mut sells: Vec<usize> = Vec::new();
    let mut buy_to_sell: Vec<usize> = Vec::new(); // parallel to buys, sell candle idx (or usize::MAX)

    let mut open_buys: Vec<usize> = Vec::new(); // stack of open buy candle indices

    for i in 50..n {
        let Some(bi) = &bb[i] else { continue };
        let Some(ki) = &kc[i] else { continue };
        let bbw = (bi.upper - bi.lower) / bi.middle;

        // 20-day average BB bandwidth
        let start_ma = if i >= 20 { i - 19 } else { 0 };
        let bbw_ma: f64 = (start_ma..=i)
            .filter_map(|j| bb[j].as_ref().map(|b| (b.upper - b.lower) / b.middle))
            .sum::<f64>()
            / ((i - start_ma + 1) as f64);

        // ── Sell check (when holding any position) ──
        if !open_buys.is_empty() {
            let atr_val = (ki.upper - ki.middle) / 2.0; // 1x ATR
            let stop_dist = atr_val * 1.5;

            // Worst entry price across all open positions (for ATR stop)
            let worst_entry = open_buys
                .iter()
                .map(|&j| candles[j].close)
                .fold(f64::NEG_INFINITY, f64::max);

            let hit_mid = candles[i].close < bi.middle;
            let hit_stop = candles[i].close < worst_entry - stop_dist;

            // KDJ death cross (K crosses below D)
            let kdj_death = (|| {
                let pk = kdj.get(i - 1).and_then(|x| x.as_ref());
                let ck = kdj[i].as_ref();
                if let (Some(p), Some(c)) = (pk, ck) {
                    p.k >= p.d && c.k < c.d
                } else {
                    false
                }
            })();

            // MACD death cross (DIF crosses below DEA)
            let macd_death = (|| {
                let pm = macd.get(i - 1).and_then(|x| x.as_ref());
                let cm = macd[i].as_ref();
                if let (Some(p), Some(c)) = (pm, cm) {
                    p.dif >= p.dea && c.dif < c.dea
                } else {
                    false
                }
            })();

            if hit_mid || hit_stop || kdj_death || macd_death {
                sells.push(i);
                // Close all open positions, each maps to this sell candle
                for _ in 0..open_buys.len() {
                    buy_to_sell.push(i);
                }
                open_buys.clear();
            }
        }

        // ── Buy check ──
        let vol5 = vol_ma(candles, i, 5);
        let vol_p75 = vol_pct75(candles, i);
        let is_breakout = candles[i].close > bi.upper;
        // Volume: > 2x 5-day MA AND > 75th percentile
        let is_vol_surge = candles[i].volume as f64 > vol5 * 2.0 && candles[i].volume as f64 > vol_p75;

        if is_breakout && is_vol_surge {
            if open_buys.is_empty() {
                // Initial entry: require BB squeeze
                let is_squeeze = bbw < bbw_ma * 0.85;
                if is_squeeze {
                    buys.push(i);
                    open_buys.push(i);
                }
            } else {
                // Re-entry (add): require pullback below BB upper within last 5 bars
                let had_pullback = (i.saturating_sub(4)..=i).any(|j| {
                    bb[j]
                        .as_ref()
                        .map_or(false, |bj| candles[j].close <= bj.upper)
                });
                if had_pullback {
                    buys.push(i);
                    open_buys.push(i);
                }
            }
        }
    }

    // Unclosed positions → sentinel
    for _ in 0..open_buys.len() {
        buy_to_sell.push(usize::MAX);
    }

    (buys, sells, buy_to_sell)
}

/// Decompose compound tags into atomic tokens, dedup, and join with " & ".
///
/// e.g. ["kdj↑macd↑", "kdj↑支撑"] → "kdj↑ & macd↑ & 支撑"
fn decompose_and_join(raw: &[&'static str]) -> String {
    // Expand known compound tags into individual atoms.
    let atoms: Vec<&'static str> = raw
        .iter()
        .flat_map(|t| match *t {
            "kdj↑macd↑" => vec!["kdj↑", "macd↑"],
            "kdj↑支撑" => vec!["kdj↑", "支撑"],
            "macd↑支撑" => vec!["macd↑", "支撑"],
            "kdj↓macd↓" => vec!["kdj↓", "macd↓"],
            "kdj↓压力" => vec!["kdj↓", "压力"],
            "macd↓压力" => vec!["macd↓", "压力"],
            // Pass-through for non-compound tags (e.g. J-line signals).
            other => vec![other],
        })
        .collect();

    // Dedup and join.
    let mut seen: Vec<&'static str> = Vec::new();
    for atom in &atoms {
        if !seen.contains(atom) {
            seen.push(atom);
        }
    }
    seen.join(" & ")
}

// ── Merge all signals ─────────────────────────────────────────────────────

pub fn compute_signals(
    candles: &[Candle],
    bb: &[Option<BollingerPoint>],
    kc: &[Option<KeltnerPoint>],
    macd: &[Option<MacdPoint>],
    kdj: &[Option<KdjPoint>],
) -> Vec<Signal> {
    let ks = kdj_signals(kdj);
    let ms = macd_signals(macd);
    let ss = sr_signals(candles, bb, kc);
    let mut out = Vec::new();

    // ── Collect all cross-confirmed buy/sell events ──
    let mut buy_events: Vec<(usize, Vec<&'static str>)> = Vec::new(); // (idx, reasons)
    let mut sell_events: Vec<(usize, Vec<&'static str>)> = Vec::new(); // (idx, reasons)

    // Confirmed buys
    for &k in &ks.kd_buy {
        if in_window(k, CONFIRM_WINDOW, |j| ms.buy_at.contains(&j)) {
            buy_events.push((k, vec!["kdj↑macd↑"]));
        }
    }
    for &k in &ks.kd_buy {
        if in_window(k, CONFIRM_WINDOW, |j| ss.buy_at.contains(&j)) {
            buy_events.push((k, vec!["kdj↑支撑"]));
        }
    }
    for &m in &ms.buy_at {
        if in_window(m, CONFIRM_WINDOW, |j| ss.buy_at.contains(&j)) {
            buy_events.push((m, vec!["macd↑支撑"]));
        }
    }

    // Confirmed sells
    for &k in &ks.kd_sell {
        if in_window(k, CONFIRM_WINDOW, |j| ms.sell_at.contains(&j)) {
            sell_events.push((k, vec!["kdj↓macd↓"]));
        }
    }
    for &k in &ks.kd_sell {
        if in_window(k, CONFIRM_WINDOW, |j| ss.sell_at.contains(&j)) {
            sell_events.push((k, vec!["kdj↓压力"]));
        }
    }
    for &m in &ms.sell_at {
        if in_window(m, CONFIRM_WINDOW, |j| ss.sell_at.contains(&j)) {
            sell_events.push((m, vec!["macd↓压力"]));
        }
    }

    // ── Step 1: Merge same-day buy events into one ──
    buy_events.sort_by_key(|(idx, _)| *idx);
    let mut merged_buys: Vec<(usize, Vec<&'static str>)> = Vec::new();
    for (idx, reasons) in buy_events {
        match merged_buys.last_mut() {
            Some((last_idx, last_reasons)) if *last_idx == idx => {
                for r in reasons {
                    if !last_reasons.contains(&r) {
                        last_reasons.push(r);
                    }
                }
            }
            _ => merged_buys.push((idx, reasons)),
        }
    }

    // ── Step 2: Merge same-day sell events into one ──
    sell_events.sort_by_key(|(idx, _)| *idx);
    let mut merged_sells: Vec<(usize, Vec<&'static str>)> = Vec::new();
    for (idx, reasons) in sell_events {
        match merged_sells.last_mut() {
            Some((last_idx, last_reasons)) if *last_idx == idx => {
                for r in reasons {
                    if !last_reasons.contains(&r) {
                        last_reasons.push(r);
                    }
                }
            }
            _ => merged_sells.push((idx, reasons)),
        }
    }

    // ── Step 3: Walk chronologically, pair sells with ALL open buys ──
    let mut open: Vec<(usize, Vec<&'static str>)> = Vec::new(); // stack of open buys
    let mut pairs: Vec<(
        Vec<(usize, Vec<&'static str>)>,
        &'static str,
        usize,
        Vec<&'static str>,
    )> = Vec::new();

    let mut bi = 0;
    let mut si = 0;
    while bi < merged_buys.len() || si < merged_sells.len() {
        let buy_ts = if bi < merged_buys.len() {
            candles[merged_buys[bi].0].time
        } else {
            i64::MAX
        };
        let sell_ts = if si < merged_sells.len() {
            candles[merged_sells[si].0].time
        } else {
            i64::MAX
        };

        if buy_ts < sell_ts {
            // Buy comes first
            open.push(merged_buys[bi].clone());
            bi += 1;
        } else {
            // Sell comes first (or same time — sell AFTER buy)
            let (sell_idx, sell_reasons) = merged_sells[si].clone();
            let exit = candles[sell_idx].close;
            if !open.is_empty() {
                // Close ALL open buys, pair with this sell
                pairs.push((open.clone(), "", sell_idx, sell_reasons.clone()));
                open.clear();
            } else {
                // Unpaired sell
                let tags = decompose_and_join(&sell_reasons.iter().copied().collect::<Vec<_>>());
                out.push(Signal {
                    time: candles[sell_idx].time,
                    kind: SignalKind::Sell,
                    price: exit,
                    reason: format!("卖({})", tags),
                    pnl_pct: None,
                });
            }
            si += 1;
        }
    }

    // ── Step 4: Emit all paired signals ──
    for (buy_entries, _sep, sell_idx, sell_reasons) in &pairs {
        let exit = candles[*sell_idx].close;

        // Flatten & dedup buy/sell tags
        let buy_tags = decompose_and_join(&buy_entries.iter().flat_map(|(_, rs)| rs.iter().copied()).collect::<Vec<_>>());
        let sell_tags = decompose_and_join(&sell_reasons.iter().copied().collect::<Vec<_>>());

        let buy_idx = buy_entries[0].0;
        let total_entry = buy_entries
            .iter()
            .map(|(i, _)| candles[*i].close)
            .sum::<f64>()
            / buy_entries.len() as f64;
        let pnl = (exit - total_entry) / total_entry * 100.0;

        out.push(Signal {
            time: candles[buy_idx].time,
            kind: SignalKind::Buy,
            price: total_entry,
            reason: format!(
                "买({}) @{:.2}→{:.2}({:+.1}%)",
                buy_tags, total_entry, exit, pnl
            ),
            pnl_pct: Some(pnl),
        });
        out.push(Signal {
            time: candles[*sell_idx].time,
            kind: SignalKind::Sell,
            price: exit,
            reason: format!("卖({})", sell_tags),
            pnl_pct: None,
        });
    }

    // ── Remaining open buys (no sell to close) ──
    if !open.is_empty() {
        let tags = decompose_and_join(&open.iter().flat_map(|(_, rs)| rs.iter().copied()).collect::<Vec<_>>());
        let buy_idx = open[0].0;
        let total_entry = open.iter().map(|(i, _)| candles[*i].close).sum::<f64>() / open.len() as f64;
        out.push(Signal {
            time: candles[buy_idx].time,
            kind: SignalKind::Buy,
            price: total_entry,
            reason: format!("买({}) @{:.2}→持仓中", tags, total_entry),
            pnl_pct: None,
        });
    }

    // ── Direct J-line signals ──
    for &j in &ks.j_buy {
        out.push(Signal {
            time: candles[j].time,
            kind: SignalKind::Buy,
            price: candles[j].close,
            reason: "KDJ J超卖(买)".into(),
            pnl_pct: None,
        });
    }
    for &j in &ks.j_sell {
        out.push(Signal {
            time: candles[j].time,
            kind: SignalKind::Sell,
            price: candles[j].close,
            reason: "KDJ J超买(卖)".into(),
            pnl_pct: None,
        });
    }

    out
}