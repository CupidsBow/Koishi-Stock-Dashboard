pub mod adx;
pub mod bollinger;
pub mod kdj;
pub mod keltner;
pub mod macd;
pub mod regime;
pub mod rsi;
pub mod signals;

// Re-exports for convenient flat access
pub use adx::adx;
pub use bollinger::bollinger_bands;
pub use kdj::kdj;
pub use keltner::keltner_channels;
pub use macd::macd;
pub use regime::market_regime;
pub use rsi::rsi;
pub use signals::compute_signals;

pub(crate) fn wilder_smooth(prev: f64, cur: f64, n: usize) -> f64 {
  (prev * (n - 1) as f64 + cur) / n as f64
}
