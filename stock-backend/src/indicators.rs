use crate::api::Candle;

fn wilder_smooth(prev: f64, cur: f64, n: usize) -> f64 {
  (prev * (n - 1) as f64 + cur) / n as f64
}

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
  if n < period {
    return result;
  }
  for i in (period - 1)..n {
    let s = &candles[i + 1 - period..=i];
    let sum: f64 = s.iter().map(|c| c.close).sum();
    let mid = sum / period as f64;
    let var: f64 = s.iter().map(|c| (c.close - mid).powi(2)).sum::<f64>() / period as f64;
    result[i] = Some(BollingerPoint {
      time: candles[i].time,
      upper: mid + 2.0 * var.sqrt(),
      middle: mid,
      lower: mid - 2.0 * var.sqrt(),
    });
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
  let ep = 20usize;
  let ap = 10usize;
  let mult = 2.0;
  let mut result = vec![None; n];
  if n < ep.max(ap) + 1 {
    return result;
  }
  let ea = 2.0 / (ep as f64 + 1.0);
  let mut ema = vec![0.0f64; n];
  ema[ep - 1] = candles[..ep].iter().map(|c| c.close).sum::<f64>() / ep as f64;
  for i in ep..n {
    ema[i] = candles[i].close * ea + ema[i - 1] * (1.0 - ea);
  }
  let mut atr = vec![0.0f64; n];
  for i in 1..=ap {
    atr[ap] += (candles[i].high - candles[i].low)
      .abs()
      .max((candles[i].high - candles[i - 1].close).abs())
      .max((candles[i].low - candles[i - 1].close).abs());
  }
  atr[ap] /= ap as f64;
  for i in ap + 1..n {
    let tr = (candles[i].high - candles[i].low)
      .abs()
      .max((candles[i].high - candles[i - 1].close).abs())
      .max((candles[i].low - candles[i - 1].close).abs());
    atr[i] = wilder_smooth(atr[i - 1], tr, ap);
  }
  let start = ep.max(ap + 1);
  for i in start..n {
    result[i] = Some(KeltnerPoint {
      time: candles[i].time,
      upper: ema[i] + mult * atr[i],
      middle: ema[i],
      lower: ema[i] - mult * atr[i],
    });
  }
  result
}

// ── MACD ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct MacdPoint {
  pub time: i64,
  pub dif: f64,
  pub dea: f64,
  pub bar: f64,
}

pub fn macd(candles: &[Candle]) -> Vec<Option<MacdPoint>> {
  let n = candles.len();
  let mut result = vec![None; n];
  if n < 26 + 9 {
    return result;
  }
  let a12 = 2.0 / 13.0;
  let a26 = 2.0 / 27.0;
  let a9 = 2.0 / 10.0;
  let mut e12 = vec![0.0f64; n];
  let mut e26 = vec![0.0f64; n];
  e12[11] = candles[..12].iter().map(|c| c.close).sum::<f64>() / 12.0;
  e26[25] = candles[..26].iter().map(|c| c.close).sum::<f64>() / 26.0;
  for i in 12..n {
    e12[i] = candles[i].close * a12 + e12[i - 1] * (1.0 - a12);
  }
  for i in 26..n {
    e26[i] = candles[i].close * a26 + e26[i - 1] * (1.0 - a26);
  }
  let mut dif = vec![0.0f64; n];
  for i in 25..n {
    dif[i] = e12[i] - e26[i];
  }
  let mut dea = vec![0.0f64; n];
  dea[33] = dif[25..=33].iter().sum::<f64>() / 9.0;
  for i in 34..n {
    dea[i] = dif[i] * a9 + dea[i - 1] * (1.0 - a9);
  }
  for i in 33..n {
    result[i] = Some(MacdPoint {
      time: candles[i].time,
      dif: dif[i],
      dea: dea[i],
      bar: 2.0 * (dif[i] - dea[i]),
    });
  }
  result
}

// ── KDJ ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct KdjPoint {
  pub time: i64,
  pub k: f64,
  pub d: f64,
  pub j: f64,
}

pub fn kdj(candles: &[Candle], period: usize) -> Vec<Option<KdjPoint>> {
  let n = candles.len();
  let mut result = vec![None; n];
  if n < period + 1 {
    return result;
  }
  let ak = 1.0 / 3.0;
  let ad = 1.0 / 3.0;
  let mut kp = 50.0;
  let mut dp = 50.0;
  for i in (period - 1)..n {
    let s = &candles[i + 1 - period..=i];
    let hn = s.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
    let ln = s.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
    let rsv = if (hn - ln).abs() < 1e-9 {
      50.0
    } else {
      (candles[i].close - ln) / (hn - ln) * 100.0
    };
    let k = ak * rsv + (1.0 - ak) * kp;
    let d = ad * k + (1.0 - ad) * dp;
    let j = 3.0 * k - 2.0 * d;
    result[i] = Some(KdjPoint {
      time: candles[i].time,
      k,
      d,
      j,
    });
    kp = k;
    dp = d;
  }
  result
}

// ── ADX ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdxPoint {
  pub time: i64,
  pub adx: f64,
  pub plus_di: f64,
  pub minus_di: f64,
}

pub fn adx(candles: &[Candle], period: usize) -> Vec<Option<AdxPoint>> {
  let n = candles.len();
  let mut result = vec![None; n];
  if n < period * 2 + 1 {
    return result;
  }
  let mut tr = vec![0.0f64; n];
  let mut pdm = vec![0.0f64; n];
  let mut ndm = vec![0.0f64; n];
  for i in 1..n {
    tr[i] = (candles[i].high - candles[i].low)
      .abs()
      .max((candles[i].high - candles[i - 1].close).abs())
      .max((candles[i].low - candles[i - 1].close).abs());
    let up = candles[i].high - candles[i - 1].high;
    let dn = candles[i - 1].low - candles[i].low;
    pdm[i] = if up > dn && up > 0.0 { up } else { 0.0 };
    ndm[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
  }
  let mut atr_s = vec![0.0f64; n];
  let mut pdm_s = vec![0.0f64; n];
  let mut ndm_s = vec![0.0f64; n];
  for i in 1..=period {
    atr_s[period] += tr[i];
    pdm_s[period] += pdm[i];
    ndm_s[period] += ndm[i];
  }
  atr_s[period] /= period as f64;
  pdm_s[period] /= period as f64;
  ndm_s[period] /= period as f64;
  for i in period + 1..n {
    atr_s[i] = wilder_smooth(atr_s[i - 1], tr[i], period);
    pdm_s[i] = wilder_smooth(pdm_s[i - 1], pdm[i], period);
    ndm_s[i] = wilder_smooth(ndm_s[i - 1], ndm[i], period);
  }
  let mut dx = vec![0.0f64; n];
  for i in period..n {
    let pdi = if atr_s[i] > 0.0 {
      100.0 * pdm_s[i] / atr_s[i]
    } else {
      0.0
    };
    let ndi = if atr_s[i] > 0.0 {
      100.0 * ndm_s[i] / atr_s[i]
    } else {
      0.0
    };
    let sum = pdi + ndi;
    dx[i] = if sum > 0.0 {
      100.0 * (pdi - ndi).abs() / sum
    } else {
      0.0
    };
  }
  let mut adx_s = vec![0.0f64; n];
  let sa = period * 2;
  adx_s[sa - 1] = dx[sa - period..sa].iter().sum::<f64>() / period as f64;
  for i in sa..n {
    adx_s[i] = wilder_smooth(adx_s[i - 1], dx[i], period);
  }
  for i in sa..n {
    let pdi = if atr_s[i] > 0.0 {
      100.0 * pdm_s[i] / atr_s[i]
    } else {
      0.0
    };
    let ndi = if atr_s[i] > 0.0 {
      100.0 * ndm_s[i] / atr_s[i]
    } else {
      0.0
    };
    result[i] = Some(AdxPoint {
      time: candles[i].time,
      adx: adx_s[i],
      plus_di: pdi,
      minus_di: ndi,
    });
  }
  result
}

// ── RSI ───────────────────────────────────────────────────────────────────

pub fn rsi(candles: &[Candle], period: usize) -> Vec<Option<f64>> {
  let n = candles.len();
  let mut result = vec![None; n];
  if n < period + 1 {
    return result;
  }
  let mut avg_g = 0.0f64;
  let mut avg_l = 0.0f64;
  for i in 1..=period {
    let ch = candles[i].close - candles[i - 1].close;
    if ch > 0.0 {
      avg_g += ch
    } else {
      avg_l += ch.abs()
    }
  }
  avg_g /= period as f64;
  avg_l /= period as f64;
  if avg_l < 1e-9 {
    result[period] = Some(100.0);
  } else {
    result[period] = Some(100.0 - 100.0 / (1.0 + avg_g / avg_l));
  }
  for i in period + 1..n {
    let ch = candles[i].close - candles[i - 1].close;
    let g = if ch > 0.0 { ch } else { 0.0 };
    let l = if ch < 0.0 { ch.abs() } else { 0.0 };
    avg_g = wilder_smooth(avg_g, g, period);
    avg_l = wilder_smooth(avg_l, l, period);
    if avg_l < 1e-9 {
      result[i] = Some(100.0);
    } else {
      result[i] = Some(100.0 - 100.0 / (1.0 + avg_g / avg_l));
    }
  }
  result
}

// ── Market Regime ─────────────────────────────────────────────────────────

pub fn market_regime(
  adx_data: &[Option<AdxPoint>],
  bb_data: &[Option<BollingerPoint>],
  candles: &[Candle],
) -> String {
  let n = candles.len();
  if n < 60 {
    return "数据不足".into();
  }
  let last_adx = adx_data
    .iter()
    .rev()
    .find_map(|x| x.as_ref().map(|a| a.adx))
    .unwrap_or(0.0);
  let prev_adx = adx_data
    .iter()
    .rev()
    .skip(3)
    .find_map(|x| x.as_ref().map(|a| a.adx))
    .unwrap_or(last_adx);
  let bbw: Vec<f64> = bb_data
    .iter()
    .filter_map(|x| x.as_ref().map(|b| (b.upper - b.lower) / b.middle))
    .collect();
  let bbw_now = bbw.last().copied().unwrap_or(0.02);
  let bbw_ma = if bbw.len() >= 20 {
    bbw.iter().rev().take(20).sum::<f64>() / 20.0
  } else {
    bbw_now
  };
  let ma10 = candles.iter().rev().take(10).map(|c| c.close).sum::<f64>() / 10.0;
  let ma10_p = candles
    .iter()
    .rev()
    .skip(3)
    .take(10)
    .map(|c| c.close)
    .sum::<f64>()
    / 10.0;
  let slope = if ma10_p > 0.0 {
    (ma10 - ma10_p) / ma10_p
  } else {
    0.0
  };
  if last_adx < 15.0 && bbw_now < bbw_ma * 0.75 {
    "窄幅整理".into()
  } else if last_adx < 20.0 {
    "震荡市".into()
  } else if last_adx > 25.0 && slope > 0.003 && bbw_now > bbw_ma {
    "单边牛市".into()
  } else if last_adx > 25.0 && slope < -0.003 && bbw_now > bbw_ma {
    "单边熊市".into()
  } else if last_adx > 30.0 && last_adx < prev_adx {
    "趋势衰竭".into()
  } else {
    "震荡市".into()
  }
}

// ── Per-pane Signals + Cross-confirmation ────────────────────────────────

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum SignalKind {
  Buy,
  Sell,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Signal {
  pub time: i64,
  pub kind: SignalKind,
  pub price: f64,
  pub reason: String,
}

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
        let tags = dedup_tags(sell_reasons.iter().copied());
        out.push(Signal {
          time: candles[sell_idx].time,
          kind: SignalKind::Sell,
          price: exit,
          reason: format!("卖({})", tags),
        });
      }
      si += 1;
    }
  }

  // ── Step 4: Emit all paired signals ──
  for (buy_entries, _sep, sell_idx, sell_reasons) in &pairs {
    let exit = candles[*sell_idx].close;

    // Flatten & dedup buy/sell tags
    let buy_tags = dedup_tags(buy_entries.iter().flat_map(|(_, rs)| rs.iter().copied()));
    let sell_tags = dedup_tags(sell_reasons.iter().copied());

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
    });
    out.push(Signal {
      time: candles[*sell_idx].time,
      kind: SignalKind::Sell,
      price: exit,
      reason: format!("卖({})", sell_tags),
    });
  }

  // ── Remaining open buys (no sell to close) ──
  if !open.is_empty() {
    let tags = dedup_tags(open.iter().flat_map(|(_, rs)| rs.iter().copied()));
    let buy_idx = open[0].0;
    let total_entry = open.iter().map(|(i, _)| candles[*i].close).sum::<f64>() / open.len() as f64;
    out.push(Signal {
      time: candles[buy_idx].time,
      kind: SignalKind::Buy,
      price: total_entry,
      reason: format!("买({}) @{:.2}→持仓中", tags, total_entry),
    });
  }

  // ── Direct J-line signals ──
  for &j in &ks.j_buy {
    out.push(Signal {
      time: candles[j].time,
      kind: SignalKind::Buy,
      price: candles[j].close,
      reason: "KDJ J超卖(买)".into(),
    });
  }
  for &j in &ks.j_sell {
    out.push(Signal {
      time: candles[j].time,
      kind: SignalKind::Sell,
      price: candles[j].close,
      reason: "KDJ J超买(卖)".into(),
    });
  }

  out
}

/// Dedup tags, join with " & "
fn dedup_tags(tags: impl Iterator<Item = &'static str>) -> String {
  let mut seen: Vec<&'static str> = Vec::new();
  for t in tags {
    if !seen.contains(&t) {
      seen.push(t);
    }
  }
  seen.join(" & ")
}
