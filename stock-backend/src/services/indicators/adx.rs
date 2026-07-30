use crate::models::{AdxPoint, Candle};

use super::wilder_smooth;

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
