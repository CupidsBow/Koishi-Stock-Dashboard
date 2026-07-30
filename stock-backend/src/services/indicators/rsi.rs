use crate::models::Candle;

use super::wilder_smooth;

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
