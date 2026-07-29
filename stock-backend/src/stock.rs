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

