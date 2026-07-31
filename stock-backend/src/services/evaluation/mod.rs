//! Factor evaluation — Stage 2 of the prediction pipeline.
//!
//! Computes rolling Information Coefficient (IC) and Information Ratio (IR)
//! for each factor, then screens valid factors and assigns per-factor weights
//! based on historical predictive power.

pub mod ic;
pub mod screen;

pub use ic::rolling_ic;
pub use screen::evaluate_all;