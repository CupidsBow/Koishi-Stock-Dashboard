use akshare::AkShareClient;
use anyhow::{Context, Result};

use crate::api::{Candle, StockInfo};

/// Fetch daily candlestick data for a Chinese A-stock symbol.
///
/// `symbol` should be the 6-digit stock code (e.g. "000001" for 平安银行).
/// `days` controls how many recent trading days to return.
pub async fn fetch_stock_data(symbol: &str, days: usize) -> Result<Vec<Candle>> {
  let client = AkShareClient::new();

  let points = client
    .a_share_candles(symbol, "qfq", days)
    .await
    .context("akshare::a_share_candles failed")?;

  let candles: Vec<Candle> = points
    .into_iter()
    .filter_map(|p| {
      let time = date_str_to_timestamp(&p.trade_date)?;
      Some(Candle {
        time,
        open: p.open,
        high: p.high,
        low: p.low,
        close: p.close,
        volume: p.volume as f64,
      })
    })
    .collect();

  Ok(candles)
}

/// Fetch intraday minute OHLCV data for the latest trading day.
///
/// `symbol` should be the 6-digit stock code.
/// `period` is "5", "15", "30", or "60" for minute intervals.
pub async fn fetch_minute_data(symbol: &str, period: &str) -> Result<Vec<Candle>> {
  let client = AkShareClient::new();

  let points = client
    .stock_zh_a_minute(symbol, period)
    .await
    .context("akshare::stock_zh_a_minute failed")?;

  let candles: Vec<Candle> = points
    .into_iter()
    .filter_map(|p| {
      let time = datetime_str_to_timestamp(&p.datetime)?;
      Some(Candle {
        time,
        open: p.open,
        high: p.high,
        low: p.low,
        close: p.close,
        volume: p.volume as f64,
      })
    })
    .collect();

  Ok(candles)
}

/// Search stocks by keyword (code or name fragment).
pub async fn search_stocks(keyword: &str) -> Result<Vec<StockInfo>> {
  let client = AkShareClient::new();

  let results = client
    .a_share_search(keyword, None, 20)
    .await
    .context("akshare::a_share_search failed")?;

  let infos: Vec<StockInfo> = results
    .into_iter()
    .map(|r| StockInfo {
      symbol: r.symbol,
      name: r.name,
      market: r.market,
    })
    .collect();

  Ok(infos)
}

/// Convert a date string like "2024-01-15" to a Unix timestamp (seconds).
fn date_str_to_timestamp(s: &str) -> Option<i64> {
  chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
    .ok()?
    .and_hms_opt(0, 0, 0)?
    .and_utc()
    .timestamp()
    .into()
}

/// Convert a datetime string like "2024-01-15 09:35:00" to a Unix timestamp (seconds).
fn datetime_str_to_timestamp(s: &str) -> Option<i64> {
  chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
    .ok()?
    .and_utc()
    .timestamp()
    .into()
}
