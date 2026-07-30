use anyhow::{Context, Result};
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::interval;

/// ── Connection Pool ────────────────────────────────────────────────────────

/// Initialise the PostgreSQL connection pool from `DATABASE_URL` in the
/// environment (or a `.env` file at the crate root).
///
/// # Usage
///
/// ```no_run
/// let pool = db::init_pool().await?;
/// ```
pub async fn init_pool() -> Result<PgPool> {
    dotenvy::dotenv().ok();

    let url =
        std::env::var("DATABASE_URL").context("DATABASE_URL must be set (in .env or environment)")?;

    let pool = PgPool::connect(&url)
        .await
        .context("Failed to connect to PostgreSQL")?;

    println!("Connected to PostgreSQL");

    Ok(pool)
}

/// ── Health Check ───────────────────────────────────────────────────────────

/// Periodically ping the database to verify the connection is still alive.
///
/// Spawns a background task that runs `SELECT 1` every `interval` seconds.
/// On failure it prints a warning to stderr (the pool handles reconnection
/// transparently — this is purely a monitoring convenience).
pub fn spawn_health_check(pool: PgPool, interval_secs: u64) {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(interval_secs));
        loop {
            tick.tick().await;
            match sqlx::query("SELECT 1").execute(&pool).await {
                Ok(_) => println!("DB health check ok"),
                Err(e) => eprintln!("DB health check FAILED: {e}"),
            }
        }
    });
}

/// ── Query Helpers ──────────────────────────────────────────────────────────

/// Fetch a single row and map it with `f`.  Returns `Ok(None)` on no rows.
pub async fn fetch_optional<T, F>(
    pool: &PgPool,
    query: &str,
    f: F,
) -> Result<Option<T>>
where
    F: FnOnce(&sqlx::postgres::PgRow) -> Result<T, sqlx::Error> + Send,
{
    let row = sqlx::query(query)
        .fetch_optional(pool)
        .await
        .context("db query failed")?;
    match row {
        Some(r) => f(&r).map(Some).context("row mapping failed"),
        None => Ok(None),
    }
}

/// Execute a write query and return the number of affected rows.
pub async fn execute(pool: &PgPool, query: &str) -> Result<u64> {
    let result = sqlx::query(query)
        .execute(pool)
        .await
        .context("db execute failed")?;
    Ok(result.rows_affected())
}

/// ── Application State ──────────────────────────────────────────────────────

/// Convenience wrapper that bundles the connection pool so Axum handlers can
/// extract it via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

impl AppState {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}