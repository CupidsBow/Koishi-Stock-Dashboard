use serde::{Deserialize, Deserializer, Serialize};

// ── Core Domain Types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
  pub time: i64,
  pub open: f64,
  pub high: f64,
  pub low: f64,
  pub close: f64,
  pub volume: f64,
}

// ── Factor Model Types ──────────────────────────────────────────────────────

/// Configuration for a single factor (name + expression + metadata).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorDef {
  pub name: String,
  pub category: String,
  pub expression: String,
  /// 1 = positive (higher → more bullish)
  /// -1 = negative (higher → more bearish)
  /// 0 = auto-detect from IC direction
  pub direction: i32,
}

impl FactorDef {
  pub fn new(
    name: impl Into<String>,
    category: impl Into<String>,
    expression: impl Into<String>,
    direction: i32,
  ) -> Self {
    Self {
      name: name.into(),
      category: category.into(),
      expression: expression.into(),
      direction,
    }
  }
}

/// Evaluation metrics for a single factor (IC, IR, validated weight).
#[derive(Debug, Clone, Serialize)]
pub struct FactorEval {
  pub name: String,
  /// Mean IC over the rolling window
  pub ic_mean: f64,
  /// Information Ratio = ic_mean / ic_std
  pub ir: f64,
  /// Effective weight (= |ic_mean| for valid factors, 0 otherwise)
  pub weight: f64,
  /// Whether this factor passes the validity threshold (|IR| > 0.3 or |ic_mean| > 0.02)
  pub is_valid: bool,
  /// Rolling IC series for frontend chart display
  pub ic_series: Vec<Option<f64>>,
}

/// Composite alpha score for one time point — this IS the prediction output.
///
/// The `total` field is an IC-weighted linear estimate of expected forward N-day
/// excess return. A higher total means a stronger bullish expectation.
#[derive(Debug, Clone, Serialize)]
pub struct AlphaScore {
  pub time: i64,
  /// Momentum-factor cluster score
  pub momentum: f64,
  /// Volatility-factor cluster score
  pub volatility: f64,
  /// Volume-factor cluster score
  pub volume: f64,
  /// Trend-factor cluster score
  pub trend: f64,
  /// Composite alpha = momentum + volatility + volume + trend
  pub total: f64,
}

// ── Deserialization helpers ─────────────────────────────────────────────────

/// Deserialize an f64 from either a number or a string — Polars JSON output may
/// produce string-encoded floats for some numeric columns.
pub fn deser_f64<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
  #[derive(Deserialize)]
  #[serde(untagged)]
  enum NumOrStr {
    Num(f64),
    Str(String),
  }
  match NumOrStr::deserialize(d)? {
    NumOrStr::Num(v) => Ok(v),
    NumOrStr::Str(s) => s.parse::<f64>().map_err(serde::de::Error::custom),
  }
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

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
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
