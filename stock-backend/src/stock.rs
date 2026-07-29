use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::api::{Candle, StockInfo};

/// Map a 6-digit A-share symbol to the Tencent market prefix.
///
/// - 6xxxxx → `sh`  (Shanghai)
/// - 0xxxxx / 3xxxxx → `sz`  (Shenzhen / ChiNext)
fn tencent_symbol(symbol: &str) -> Result<String> {
    if symbol.len() != 6 {
        bail!("invalid A-share symbol length: {symbol}");
    }
    match symbol.chars().next().unwrap() {
        '6' => Ok(format!("sh{symbol}")),
        '0' | '3' => Ok(format!("sz{symbol}")),
        c => bail!("unknown A-share market prefix: {c}"),
    }
}

/// Parse a Tencent kline row: `["2024-01-15", "10.50", "10.80", "10.90", "10.40", "12345678"]`
fn parse_tencent_row(row: &[serde_json::Value]) -> Option<Candle> {
    let time = date_str_to_timestamp(row.first()?.as_str()?)?;
    Some(Candle {
        time,
        open: row.get(1)?.as_str()?.parse().ok()?,
        close: row.get(2)?.as_str()?.parse().ok()?,
        high: row.get(3)?.as_str()?.parse().ok()?,
        low: row.get(4)?.as_str()?.parse().ok()?,
        volume: row.get(5)?.as_str()?.parse::<f64>().ok()?,
    })
}

/// Fetch daily candlestick data for a Chinese A-stock symbol.
///
/// Uses Tencent Finance kline API (`web.ifzq.gtimg.cn`).
/// `symbol` should be the 6-digit stock code (e.g. "000001" for 平安银行).
/// `days` controls how many recent trading days to return.
pub async fn fetch_stock_data(symbol: &str, days: usize) -> Result<Vec<Candle>> {
    let ts = tencent_symbol(symbol)?;
    let url = format!(
        "https://web.ifzq.gtimg.cn/appstock/app/fqkline/get?param={ts},day,,,{days},qfq"
    );

    let resp: serde_json::Value = reqwest::get(&url)
        .await
        .context("Tencent kline request failed")?
        .json()
        .await
        .context("Tencent kline response decode failed")?;

    let klines = resp["data"][&ts]["qfqday"]
        .as_array()
        .or_else(|| resp["data"][&ts]["day"].as_array())
        .context("Tencent kline: missing qfqday/day array")?;

    let candles: Vec<Candle> = klines.iter().filter_map(|row| parse_tencent_row(row.as_array()?)).collect();

    if candles.is_empty() {
        bail!("Tencent returned no candle data for {symbol}");
    }

    Ok(candles)
}

/// Search stocks by keyword (code or name fragment).
///
/// Uses Eastmoney suggest API (`searchapi.eastmoney.com`).
pub async fn search_stocks(keyword: &str) -> Result<Vec<StockInfo>> {
    let url = format!(
        "https://searchapi.eastmoney.com/api/suggest/get?input={}&type=14&token=D43BF722C8E33BDC906FB84D85E326E8&count=20",
        urlencoding(keyword)
    );

    #[derive(Deserialize)]
    struct SearchItem {
        #[serde(rename = "Code")]
        code: String,
        #[serde(rename = "Name")]
        name: String,
        #[serde(rename = "SecurityTypeName")]
        security_type_name: String,
    }

    #[derive(Deserialize)]
    struct SearchData {
        #[serde(rename = "Data")]
        data: Vec<SearchItem>,
    }

    #[derive(Deserialize)]
    struct SearchResp {
        #[serde(rename = "QuotationCodeTable")]
        quotation_code_table: SearchData,
    }

    let resp: SearchResp = reqwest::get(&url)
        .await
        .context("Eastmoney search request failed")?
        .json()
        .await
        .context("Eastmoney search response decode failed")?;

    let infos: Vec<StockInfo> = resp
        .quotation_code_table
        .data
        .into_iter()
        .map(|r| StockInfo {
            symbol: r.code,
            name: r.name,
            market: r.security_type_name,
        })
        .collect();

    Ok(infos)
}

/// Minimal URL-encoding for search keywords (only needs to handle Chinese UTF-8).
fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for &byte in s.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => result.push(byte as char),
            _ => result.push_str(&format!("%{:02X}", byte)),
        }
    }
    result
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