//! Minimal Hyperliquid backtesting toolkit.
//!
//! This crate provides just enough building blocks to run lightweight experiments
//! in unit tests: a [`Position`] type, simple order requests, a [`FundingPayment`]
//! structure and a very small [`RiskManager`]. The implementation intentionally
//! avoids external dependencies or complex behaviours so the library can compile
//! quickly and remain easy to understand.

pub mod alpha;
pub mod backtest;
pub mod data;
pub mod errors;
pub mod features;
pub mod optimization;
pub mod report;
pub mod risk_manager;
pub mod signals;
pub mod strategy;
pub mod unified_data;

#[cfg(test)]
mod tests {
    mod alpha_pipeline_tests;
    mod basic;
}

/// Convenient re-export of the most common items used when writing examples or tests.
pub mod prelude {
    pub use crate::backtest::FundingPayment;
    pub use crate::risk_manager::{RiskConfig, RiskError, RiskManager, RiskOrder};
    pub use crate::unified_data::{
        OrderRequest, OrderResult, OrderSide, OrderType, Position, TimeInForce,
    };
}
