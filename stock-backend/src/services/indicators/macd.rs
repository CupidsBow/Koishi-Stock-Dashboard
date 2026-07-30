use crate::models::{Candle, MacdPoint};

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