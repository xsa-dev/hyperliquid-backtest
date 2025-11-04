//! Minimal Hyperliquid backtesting toolkit.
//!
//! This crate provides just enough building blocks to run lightweight experiments
//! in unit tests: a [`Position`] type, simple order requests, a [`FundingPayment`]
//! structure and a very small [`RiskManager`]. The implementation intentionally
//! avoids external dependencies or complex behaviours so the library can compile
//! quickly and remain easy to understand.

pub mod backtest;
pub mod optimization;
pub mod risk_manager;
pub mod unified_data;
pub mod data;
pub mod strategies;
pub mod errors;

#[cfg(test)]
mod tests {
    mod basic;
}

/// Convenient re-export of the most common items used when writing examples or tests.
pub mod prelude {
    pub use crate::backtest::*;
    pub use crate::data::HyperliquidData;
    pub use crate::strategies::{enhanced_sma_cross, FundingAwareConfig};
    pub use crate::risk_manager::{RiskConfig, RiskError, RiskManager, RiskOrder};
    pub use crate::unified_data::{
        OrderRequest, OrderResult, OrderSide, OrderType, Position, TimeInForce,
    };
    pub use crate::errors::{HyperliquidBacktestError, Result};
}
