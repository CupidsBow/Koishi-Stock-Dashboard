use crate::models::{Candle, KeltnerPoint};

use super::wilder_smooth;

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