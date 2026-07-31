//! Factor expression engine — Stage 1 of the prediction pipeline.
//!
//! Converts OHLCV candles into factor values. The parser validates expression
//! syntax; actual computation uses fast numeric functions in `compute.rs`.

pub mod compute;
pub mod parser;
pub mod registry;

pub use compute::compute_raw_factors;
pub use parser::{default_registry, parse_expression};
pub use registry::builtin_factors;