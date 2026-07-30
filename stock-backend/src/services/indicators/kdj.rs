use crate::models::{Candle, KdjPoint};

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