use crate::models::{BollingerPoint, Candle};

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
