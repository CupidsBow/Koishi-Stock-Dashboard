use anyhow::{Context, Result};
use chrono::{Datelike, Duration, Timelike, Utc};
use sqlx::PgPool;

use crate::models::Candle;
use crate::services::stock::fetch_stock_data;

// ── Freshness Helpers ───────────────────────────────────────────────────────

/// True when the A-share market is currently open (Mon–Fri, 9:30–15:00 CST).
///
/// Simplification: does not check Chinese holidays.  A false-positive on a
/// holiday just means a 5-minute TTL on a day with no new data — harmless.
fn market_is_open_now() -> bool {
  let now = Utc::now() + Duration::hours(8); // CST = UTC+8
  now.weekday().num_days_from_monday() <= 4
    && now.hour() >= 9
    && (now.hour() < 11
      || now.hour() == 11 && now.minute() <= 30
      || now.hour() >= 13 && (now.hour() < 15 || now.hour() == 15 && now.minute() == 0))
}

/// How long cached candles are considered fresh.
fn cache_ttl() -> chrono::Duration {
  if market_is_open_now() {
    Duration::minutes(5)
  } else {
    Duration::hours(2)
  }
}

// ── Read ────────────────────────────────────────────────────────────────────

/// Try to serve candles from the PostgreSQL cache.
///
/// Returns `Ok(Some(Vec<Candle>))` when:
/// - a row exists for `symbol`,
/// - `day_count >= min_count`,
/// - the row's `fetched_at` is still fresh (see [`cache_ttl`]).
///
/// Returns `Ok(None)` for any cache miss or staleness.  Returns `Err` only
/// on a database failure that the caller should log and fall through from.
pub async fn get_cached_candles(
  pool: &PgPool,
  symbol: &str,
  min_count: usize,
) -> Result<Option<Vec<Candle>>> {
  let row: Option<(serde_json::Value, i32, chrono::DateTime<Utc>)> = sqlx::query_as(
    "SELECT candle_data, day_count, fetched_at FROM candles_cache WHERE symbol = $1",
  )
  .bind(symbol)
  .fetch_optional(pool)
  .await
  .context("cache read query failed")?;

  let Some((candle_data, day_count, fetched_at)) = row else {
    return Ok(None);
  };

  // Not enough candles cached — treat as partial miss.
  if day_count < min_count as i32 {
    return Ok(None);
  }

  // Expired?
  let age = Utc::now() - fetched_at;
  if age > cache_ttl() {
    return Ok(None);
  }

  // Deserialise the JSONB array.
  let candles: Vec<Candle> =
    serde_json::from_value(candle_data).context("deserialising cached candles")?;

  // Return exactly the requested window (most recent N candles).
  if candles.len() > min_count {
    Ok(Some(candles[candles.len() - min_count..].to_vec()))
  } else {
    Ok(Some(candles))
  }
}

// ── Write ───────────────────────────────────────────────────────────────────

/// Upsert candles into the cache.  Idempotent — repeated calls with the same
/// symbol replace the previous row.
pub async fn store_candles(pool: &PgPool, symbol: &str, candles: &[Candle]) -> Result<()> {
  let data = serde_json::to_value(candles).context("serialising candles for cache")?;
  let day_count = candles.len() as i32;

  sqlx::query(
    "INSERT INTO candles_cache (symbol, candle_data, day_count, fetched_at)
         VALUES ($1, $2, $3, NOW())
         ON CONFLICT (symbol) DO UPDATE SET
             candle_data = EXCLUDED.candle_data,
             day_count  = EXCLUDED.day_count,
             fetched_at = NOW()",
  )
  .bind(symbol)
  .bind(&data)
  .bind(day_count)
  .execute(pool)
  .await
  .context("cache write query failed")?;

  Ok(())
}

// ── Orchestrator ────────────────────────────────────────────────────────────

/// Fetch candles for `symbol` with caching.
///
/// 1. Check the PostgreSQL cache — return immediately on a fresh hit.
/// 2. Otherwise call the Tencent Finance API via [`fetch_stock_data`].
/// 3. Kick off a background task to persist the fresh data.
///
/// Cache or persistence failures are logged and never propagate to the caller.
pub async fn get_candles_with_cache(
  pool: &PgPool,
  symbol: &str,
  days: usize,
) -> Result<Vec<Candle>> {
  // 1. Try cache.
  match get_cached_candles(pool, symbol, days).await {
    Ok(Some(candles)) => return Ok(candles),
    Ok(None) => { /* miss — fetch from API */ }
    Err(e) => eprintln!("Cache read error for {symbol}: {e}"),
  }

  // 2. Fetch from external API.
  let candles = fetch_stock_data(symbol, days).await?;

  // 3. Write to cache in background (best-effort).
  let pool_clone = pool.clone();
  let sym = symbol.to_string();
  let c = candles.clone();
  tokio::spawn(async move {
    if let Err(e) = store_candles(&pool_clone, &sym, &c).await {
      eprintln!("Cache write error for {sym}: {e}");
    }
  });

  Ok(candles)
}
