use serde::Serialize;

// ── Core Domain Types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Candle {
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Serialize)]
pub struct StockInfo {
    pub symbol: String,
    pub name: String,
    pub market: String,
}

// ── Indicator Output Types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct BollingerPoint {
    pub time: i64,
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct KeltnerPoint {
    pub time: i64,
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MacdPoint {
    pub time: i64,
    pub dif: f64,
    pub dea: f64,
    pub bar: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct KdjPoint {
    pub time: i64,
    pub k: f64,
    pub d: f64,
    pub j: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdxPoint {
    pub time: i64,
    pub adx: f64,
    pub plus_di: f64,
    pub minus_di: f64,
}

// ── Signal Types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize)]
pub enum SignalKind {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize)]
pub struct Signal {
    pub time: i64,
    pub kind: SignalKind,
    pub price: f64,
    pub reason: String,
    /// PnL percentage for closed buy signals (None for sells / unclosed /
    /// J-line signals).
    pub pnl_pct: Option<f64>,
}