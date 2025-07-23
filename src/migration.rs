//! # Migration Guide from rs-backtester
//!
//! This module provides comprehensive guidance for migrating existing rs-backtester code
//! to use hyperliquid-backtester with enhanced Hyperliquid-specific features.
//!
//! ## Overview
//!
//! hyperliquid-backtester is designed as a drop-in enhancement to rs-backtester, meaning
//! most existing code will work with minimal changes while gaining access to:
//!
//! - Real Hyperliquid market data
//! - Funding rate calculations
//! - Enhanced commission structures
//! - Perpetual futures mechanics
//! - Advanced reporting features
//!
//! ## Basic Migration Steps
//!
//! ### 1. Update Dependencies
//!
//! **Before (rs-backtester only):**
//! ```toml
//! [dependencies]
//! rs-backtester = "0.1"
//! ```
//!
//! **After (with hyperliquid-backtester):**
//! ```toml
//! [dependencies]
//! hyperliquid-backtester = "0.1"
//! tokio = { version = "1.0", features = ["full"] }
//! ```
//!
//! ### 2. Update Imports
//!
//! **Before:**
//! ```rust,ignore
//! use rs_backtester::prelude::*;
//! ```
//!
//! **After:**
//! ```rust,ignore
//! use hyperliquid_backtester::prelude::*;
//! ```
//!
//! ### 3. Replace Data Sources
//!
//! **Before (CSV or manual data):**
//! ```rust,ignore
//! let data = Data::from_csv("data.csv")?;
//! ```
//!
//! **After (Live Hyperliquid data):**
//! ```rust,ignore
//! let data = HyperliquidData::fetch("BTC", "1h", start_time, end_time).await?;
//! ```
//!
//! ## Detailed Migration Examples
//!
//! ### Example 1: Simple Moving Average Strategy
//!
//! **Before (rs-backtester):**
//! ```rust,ignore
//! use rs_backtester::prelude::*;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let data = Data::from_csv("btc_data.csv")?;
//!     
//!     let strategy = sma_cross(data.clone(), 10, 20);
//!     
//!     let mut backtest = Backtest::new(
//!         data,
//!         strategy,
//!         10000.0,
//!         Commission { rate: 0.001 },
//!     );
//!     
//!     backtest.run();
//!     let report = backtest.report();
//!     
//!     println!("Total Return: {:.2}%", report.total_return * 100.0);
//!     
//!     Ok(())
//! }
//! ```
//!
//! **After (hyperliquid-backtester):**
//! ```rust,ignore
//! use hyperliquid_backtester::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), HyperliquidBacktestError> {
//!     // Fetch real Hyperliquid data
//!     let data = HyperliquidData::fetch("BTC", "1h", start_time, end_time).await?;
//!     
//!     // Use enhanced SMA strategy with funding awareness
//!     let strategy = enhanced_sma_cross(10, 20, Default::default())?;
//!     
//!     let mut backtest = HyperliquidBacktest::new(
//!         data,
//!         strategy,
//!         10000.0,
//!         HyperliquidCommission::default(), // Realistic Hyperliquid fees
//!     )?;
//!     
//!     // Include funding calculations
//!     backtest.calculate_with_funding()?;
//!     let report = backtest.enhanced_report()?;
//!     
//!     println!("Total Return: {:.2}%", report.total_return * 100.0);
//!     println!("Funding PnL: ${:.2}", report.funding_pnl); // New!
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Example 2: Custom Strategy Migration
//!
//! **Before (rs-backtester custom strategy):**
//! ```rust,ignore
//! use rs_backtester::prelude::*;
//!
//! fn rsi_strategy(data: Data, period: usize, oversold: f64, overbought: f64) -> Strategy {
//!     let mut strategy = Strategy::new();
//!     
//!     strategy.next(Box::new(move |ctx, _| {
//!         let rsi = calculate_rsi(&ctx.data().close, period, ctx.index());
//!         
//!         if rsi < oversold {
//!             ctx.entry_qty(1.0); // Go long
//!         } else if rsi > overbought {
//!             ctx.entry_qty(-1.0); // Go short
//!         }
//!     }));
//!     
//!     strategy
//! }
//! ```
//!
//! **After (hyperliquid-backtester with funding awareness):**
//! ```rust,ignore
//! use hyperliquid_backtester::prelude::*;
//!
//! fn enhanced_rsi_strategy(
//!     data: &HyperliquidData, 
//!     period: usize, 
//!     oversold: f64, 
//!     overbought: f64,
//!     funding_config: FundingAwareConfig,
//! ) -> Result<Strategy, HyperliquidBacktestError> {
//!     let mut strategy = Strategy::new();
//!     let data_clone = data.clone();
//!     
//!     strategy.next(Box::new(move |ctx, _| {
//!         let rsi = calculate_rsi(&ctx.data().close, period, ctx.index());
//!         
//!         // Get funding rate for current timestamp
//!         let current_time = data_clone.datetime[ctx.index()];
//!         let funding_rate = data_clone.get_funding_rate_at(current_time).unwrap_or(0.0);
//!         
//!         // Combine RSI and funding signals
//!         let mut signal = 0.0;
//!         
//!         if rsi < oversold {
//!             signal = 1.0; // RSI suggests long
//!         } else if rsi > overbought {
//!             signal = -1.0; // RSI suggests short
//!         }
//!         
//!         // Adjust signal based on funding rate
//!         if funding_rate.abs() > funding_config.funding_threshold {
//!             if funding_rate > 0.0 {
//!                 // Positive funding - longs pay shorts, favor long
//!                 signal += funding_config.funding_weight;
//!             } else {
//!                 // Negative funding - shorts pay longs, favor short
//!                 signal -= funding_config.funding_weight;
//!             }
//!         }
//!         
//!         // Apply final signal
//!         if signal > 0.5 {
//!             ctx.entry_qty(1.0);
//!         } else if signal < -0.5 {
//!             ctx.entry_qty(-1.0);
//!         } else {
//!             ctx.exit();
//!         }
//!     }));
//!     
//!     Ok(strategy)
//! }
//! ```
//!
//! ## Key Differences and Enhancements
//!
//! ### 1. Async Operations
//!
//! hyperliquid-backtester uses async operations for data fetching:
//!
//! ```rust,ignore
//! // Always use #[tokio::main] for async main function
//! #[tokio::main]
//! async fn main() -> Result<(), HyperliquidBacktestError> {
//!     let data = HyperliquidData::fetch("BTC", "1h", start, end).await?;
//!     // ... rest of code
//! }
//! ```
//!
//! ### 2. Enhanced Commission Structure
//!
//! **rs-backtester:**
//! ```rust,ignore
//! Commission { rate: 0.001 } // Single rate for all trades
//! ```
//!
//! **hyperliquid-backtester:**
//! ```rust,ignore
//! HyperliquidCommission {
//!     maker_rate: 0.0002,    // 0.02% for maker orders
//!     taker_rate: 0.0005,    // 0.05% for taker orders
//!     funding_enabled: true, // Include funding calculations
//! }
//! ```
//!
//! ### 3. Enhanced Reporting
//!
//! **rs-backtester:**
//! ```rust,ignore
//! let report = backtest.report();
//! println!("Return: {:.2}%", report.total_return * 100.0);
//! ```
//!
//! **hyperliquid-backtester:**
//! ```rust,ignore
//! let report = backtest.enhanced_report()?;
//! println!("Trading Return: {:.2}%", report.trading_return * 100.0);
//! println!("Funding Return: {:.2}%", report.funding_return * 100.0);
//! println!("Total Return: {:.2}%", report.total_return * 100.0);
//! 
//! // Additional funding-specific metrics
//! let funding_report = backtest.funding_report()?;
//! println!("Avg Funding Rate: {:.4}%", funding_report.avg_funding_rate * 100.0);
//! ```
//!
//! ## Common Migration Patterns
//!
//! ### Pattern 1: Data Loading
//!
//! **From CSV files:**
//! ```rust,ignore
//! // Old
//! let data = Data::from_csv("data.csv")?;
//!
//! // New - convert existing CSV to HyperliquidData format
//! let data = HyperliquidData::from_csv("data.csv").await?;
//! // Or fetch fresh data
//! let data = HyperliquidData::fetch("BTC", "1h", start, end).await?;
//! ```
//!
//! ### Pattern 2: Strategy Enhancement
//!
//! **Add funding awareness to existing strategies:**
//! ```rust,ignore
//! // Wrap existing rs-backtester strategy
//! let base_strategy = your_existing_strategy();
//! let enhanced_strategy = base_strategy.with_funding_awareness(0.0001)?;
//! ```
//!
//! ### Pattern 3: Error Handling
//!
//! **Update error handling:**
//! ```rust,ignore
//! // Old
//! fn run_backtest() -> Result<(), Box<dyn std::error::Error>> {
//!     // ...
//! }
//!
//! // New
//! async fn run_backtest() -> Result<(), HyperliquidBacktestError> {
//!     // ...
//! }
//! ```
//!
//! ## Compatibility Notes
//!
//! ### What Stays the Same
//!
//! - Core strategy logic and indicators
//! - Basic backtest workflow
//! - Report structure (with additions)
//! - CSV export functionality (enhanced)
//!
//! ### What Changes
//!
//! - Data fetching becomes async
//! - Commission structure is more detailed
//! - Error types are more specific
//! - Additional funding-related methods
//!
//! ### Breaking Changes
//!
//! 1. **Async requirement**: Data fetching requires async context
//! 2. **Error types**: `HyperliquidBacktestError` instead of generic errors
//! 3. **Commission structure**: More detailed fee structure
//! 4. **Import paths**: Use `hyperliquid_backtester::prelude::*`
//!
//! ## Migration Checklist
//!
//! - [ ] Update `Cargo.toml` dependencies
//! - [ ] Change imports to `hyperliquid_backtester::prelude::*`
//! - [ ] Add `#[tokio::main]` to main function
//! - [ ] Replace data loading with `HyperliquidData::fetch()`
//! - [ ] Update commission structure to `HyperliquidCommission`
//! - [ ] Update error handling to use `HyperliquidBacktestError`
//! - [ ] Test enhanced features (funding calculations, etc.)
//! - [ ] Update documentation and examples
//!
//! ## Getting Help
//!
//! If you encounter issues during migration:
//!
//! 1. Check the [API documentation](https://docs.rs/hyperliquid-backtester)
//! 2. Review the [examples directory](https://github.com/xsa-dev/hyperliquid-backtester/tree/main/examples)
//! 3. Open an issue on [GitHub](https://github.com/xsa-dev/hyperliquid-backtester/issues)
//!
//! ## Performance Considerations
//!
//! - **Data fetching**: Cache data locally for repeated backtests
//! - **Funding calculations**: Can be disabled if not needed via `funding_enabled: false`
//! - **Memory usage**: HyperliquidData includes additional funding rate arrays
//! - **Network calls**: Batch data requests when possible
