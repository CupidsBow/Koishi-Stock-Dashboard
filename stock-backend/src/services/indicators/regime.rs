use crate::models::{AdxPoint, BollingerPoint, Candle};

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