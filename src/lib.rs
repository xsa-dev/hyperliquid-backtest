//! # Hyperliquid Backtester
//!
//! A comprehensive Rust library that integrates Hyperliquid trading data with the rs-backtester 
//! framework to enable sophisticated backtesting of trading strategies using real Hyperliquid 
//! market data, including perpetual futures mechanics and funding rate calculations.
//!
//! ## Features
//!
//! - 🚀 **Async Data Fetching**: Efficiently fetch historical OHLC data from Hyperliquid API
//! - 💰 **Funding Rate Support**: Complete funding rate data and perpetual futures mechanics
//! - 🔄 **Seamless Integration**: Drop-in replacement for rs-backtester with enhanced features
//! - 📊 **Enhanced Reporting**: Comprehensive metrics including funding PnL and arbitrage analysis
//! - ⚡ **High Performance**: Optimized for large datasets and complex multi-asset strategies
//! - 🛡️ **Type Safety**: Comprehensive error handling with detailed error messages
//! - 📈 **Advanced Strategies**: Built-in funding arbitrage and enhanced technical indicators
//!
//! ## API Stability
//!
//! This crate follows semantic versioning (SemVer):
//! - **Major version** (0.x.y → 1.0.0): Breaking API changes
//! - **Minor version** (0.1.x → 0.2.0): New features, backward compatible
//! - **Patch version** (0.1.0 → 0.1.1): Bug fixes, backward compatible
//!
//! Current version: **0.1.0** (Pre-1.0 development phase)
//!
//! ### Stability Guarantees
//!
//! - **Public API**: All items in the [`prelude`] module are considered stable within minor versions
//! - **Data Structures**: [`HyperliquidData`], [`HyperliquidBacktest`], and [`HyperliquidCommission`] are stable
//! - **Error Types**: [`HyperliquidBacktestError`] variants may be added but not removed in minor versions
//! - **Strategy Interface**: [`HyperliquidStrategy`] trait is stable for implementors
//!
//! ## Quick Start
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! hyperliquid-backtester = "0.1"
//! tokio = { version = "1.0", features = ["full"] }
//! ```
//!
//! ### Basic Backtesting Example
//!
//! ```rust,no_run
//! use hyperliquid_backtester::prelude::*;
//! use chrono::{DateTime, FixedOffset, Utc};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), HyperliquidBacktestError> {
//!     // Define time range (last 30 days)
//!     let end_time = Utc::now().timestamp() as u64;
//!     let start_time = end_time - (30 * 24 * 60 * 60); // 30 days ago
//!     
//!     // Fetch historical data for BTC with 1-hour intervals
//!     let data = HyperliquidData::fetch("BTC", "1h", start_time, end_time).await?;
//!     
//!     // Create a simple moving average crossover strategy
//!     let strategy = enhanced_sma_cross(10, 20, Default::default())?;
//!     
//!     // Set up backtest with $10,000 initial capital
//!     let mut backtest = HyperliquidBacktest::new(
//!         data,
//!         strategy,
//!         10000.0,
//!         HyperliquidCommission::default(),
//!     )?;
//!     
//!     // Run backtest including funding calculations
//!     backtest.calculate_with_funding()?;
//!     
//!     // Generate comprehensive report
//!     let report = backtest.enhanced_report()?;
//!     
//!     println!("📊 Backtest Results:");
//!     println!("Total Return: {:.2}%", report.total_return * 100.0);
//!     println!("Trading PnL: ${:.2}", report.trading_pnl);
//!     println!("Funding PnL: ${:.2}", report.funding_pnl);
//!     println!("Sharpe Ratio: {:.3}", report.sharpe_ratio);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Funding Arbitrage Strategy Example
//!
//! ```rust,no_run
//! use hyperliquid_backtester::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), HyperliquidBacktestError> {
//!     let data = HyperliquidData::fetch("ETH", "1h", start_time, end_time).await?;
//!     
//!     // Create funding arbitrage strategy with 0.01% threshold
//!     let strategy = funding_arbitrage_strategy(0.0001)?;
//!     
//!     let mut backtest = HyperliquidBacktest::new(
//!         data,
//!         strategy,
//!         50000.0, // Higher capital for arbitrage
//!         HyperliquidCommission::default(),
//!     )?;
//!     
//!     backtest.calculate_with_funding()?;
//!     
//!     // Get detailed funding analysis
//!     let funding_report = backtest.funding_report()?;
//!     
//!     println!("💰 Funding Arbitrage Results:");
//!     println!("Total Funding Received: ${:.2}", funding_report.total_funding_received);
//!     println!("Total Funding Paid: ${:.2}", funding_report.total_funding_paid);
//!     println!("Net Funding PnL: ${:.2}", funding_report.net_funding_pnl);
//!     println!("Average Funding Rate: {:.4}%", funding_report.avg_funding_rate * 100.0);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Migration from rs-backtester
//!
//! This library is designed as a drop-in enhancement to rs-backtester. See the 
//! [migration guide](https://docs.rs/hyperliquid-backtester/latest/hyperliquid_backtester/migration/index.html) 
//! for detailed instructions on upgrading existing rs-backtester code.
//!
//! ## Error Handling
//!
//! All fallible operations return [`Result<T, HyperliquidBacktestError>`](Result). The error type
//! provides detailed context and suggestions for resolution:
//!
//! ```rust,no_run
//! use hyperliquid_backtester::prelude::*;
//!
//! match HyperliquidData::fetch("INVALID", "1h", start, end).await {
//!     Ok(data) => println!("Success!"),
//!     Err(HyperliquidBacktestError::HyperliquidApi(msg)) => {
//!         eprintln!("API Error: {}", msg);
//!         // Handle API-specific errors
//!     },
//!     Err(HyperliquidBacktestError::UnsupportedInterval(interval)) => {
//!         eprintln!("Unsupported interval: {}", interval);
//!         eprintln!("Supported intervals: {:?}", HyperliquidDataFetcher::supported_intervals());
//!     },
//!     Err(e) => eprintln!("Other error: {}", e),
//! }
//! ```

pub mod data;
pub mod backtest;
pub mod strategies;
pub mod errors;
pub mod utils;
pub mod indicators;
pub mod funding_report;
pub mod csv_export;
pub mod migration;
pub mod api_docs;
pub mod trading_mode;
pub mod trading_mode_impl;
pub mod unified_data;
pub mod unified_data_impl;
pub mod paper_trading;
pub mod real_time_data_stream;
pub mod real_time_monitoring;
pub mod risk_manager;
pub mod live_trading;
pub mod live_trading_safety;
pub mod mode_reporting;

/// Logging and debugging utilities
pub mod logging {
    //! Logging and debugging utilities for the hyperliquid-backtester crate.
    //!
    //! This module provides convenient functions for setting up structured logging
    //! and debugging support throughout the library.
    //!
    //! ## Basic Usage
    //!
    //! ```rust
    //! use hyperliquid_backtester::logging::init_logger;
    //!
    //! // Initialize with default settings (INFO level)
    //! init_logger();
    //!
    //! // Or with custom log level
    //! init_logger_with_level("debug");
    //! ```
    //!
    //! ## Environment Variables
    //!
    //! You can control logging behavior using environment variables:
    //!
    //! - `RUST_LOG`: Set log level (e.g., `debug`, `info`, `warn`, `error`)
    //! - `HYPERLIQUID_LOG_FORMAT`: Set format (`json` or `pretty`)
    //! - `HYPERLIQUID_LOG_FILE`: Write logs to file instead of stdout
    //!
    //! ## Examples
    //!
    //! ```bash
    //! # Enable debug logging
    //! RUST_LOG=debug cargo run --example basic_backtest
    //!
    //! # Use JSON format for structured logging
    //! RUST_LOG=info HYPERLIQUID_LOG_FORMAT=json cargo run
    //!
    //! # Write logs to file
    //! RUST_LOG=info HYPERLIQUID_LOG_FILE=backtest.log cargo run
    //! ```

    use std::env;
    use tracing_subscriber::{
        fmt::{self, format::FmtSpan},
        layer::SubscriberExt,
        util::SubscriberInitExt,
        EnvFilter,
    };

    /// Initialize the default logger with INFO level logging.
    ///
    /// This sets up structured logging with reasonable defaults for most use cases.
    /// The logger will respect the `RUST_LOG` environment variable if set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use hyperliquid_backtester::logging::init_logger;
    ///
    /// init_logger();
    /// log::info!("Logger initialized successfully");
    /// ```
    pub fn init_logger() {
        init_logger_with_level("info");
    }

    /// Initialize the logger with a specific log level.
    ///
    /// # Arguments
    ///
    /// * `level` - The log level to use (e.g., "debug", "info", "warn", "error")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use hyperliquid_backtester::logging::init_logger_with_level;
    ///
    /// init_logger_with_level("debug");
    /// log::debug!("Debug logging enabled");
    /// ```
    pub fn init_logger_with_level(level: &str) {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(format!("hyperliquid_backtester={}", level)));

        let format = env::var("HYPERLIQUID_LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());
        let log_file = env::var("HYPERLIQUID_LOG_FILE").ok();

        let subscriber = tracing_subscriber::registry().with(env_filter);

        match (format.as_str(), log_file) {
            ("json", Some(file_path)) => {
                let file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file_path)
                    .expect("Failed to open log file");
                
                subscriber
                    .with(
                        fmt::layer()
                            .json()
                            .with_writer(file)
                            .with_span_events(FmtSpan::CLOSE)
                    )
                    .init();
            }
            ("json", None) => {
                subscriber
                    .with(
                        fmt::layer()
                            .json()
                            .with_span_events(FmtSpan::CLOSE)
                    )
                    .init();
            }
            (_, Some(file_path)) => {
                let file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file_path)
                    .expect("Failed to open log file");
                
                subscriber
                    .with(
                        fmt::layer()
                            .pretty()
                            .with_writer(file)
                            .with_span_events(FmtSpan::CLOSE)
                    )
                    .init();
            }
            _ => {
                subscriber
                    .with(
                        fmt::layer()
                            .pretty()
                            .with_span_events(FmtSpan::CLOSE)
                    )
                    .init();
            }
        }
    }

    /// Initialize logger for testing with reduced verbosity.
    ///
    /// This is useful for tests where you want to capture logs but don't want
    /// them to interfere with test output.
    pub fn init_test_logger() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_env_filter("hyperliquid_backtester=warn")
            .try_init();
    }

    /// Create a tracing span for performance monitoring.
    ///
    /// This is useful for tracking the performance of specific operations
    /// like data fetching or backtest calculations.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the operation being tracked
    /// * `details` - Additional details to include in the span
    ///
    /// # Examples
    ///
    /// ```rust
    /// use hyperliquid_backtester::logging::performance_span;
    /// use tracing::Instrument;
    ///
    /// async fn fetch_data() -> Result<(), Box<dyn std::error::Error>> {
    ///     let span = performance_span("data_fetch", &[("symbol", "BTC"), ("interval", "1h")]);
    ///     
    ///     async {
    ///         // Your data fetching logic here
    ///         Ok(())
    ///     }
    ///     .instrument(span)
    ///     .await
    /// }
    /// ```
    pub fn performance_span(name: &str, details: &[(&str, &str)]) -> tracing::Span {
        let span = tracing::info_span!("performance", operation = name);
        
        // Record additional fields if needed
        for (key, value) in details {
            span.record(*key, *value);
        }
        
        span
    }
}

#[cfg(test)]
mod tests {
    pub mod backtest_tests;
    pub mod strategies_tests;
    pub mod indicators_tests;
    pub mod indicators_tests_extended;
    pub mod funding_report_tests;
    pub mod csv_export_tests;
    pub mod csv_export_tests_enhanced;
    pub mod data_tests;
    pub mod data_price_tests;
    pub mod data_conversion_tests;
    pub mod errors_tests;
    pub mod commission_tests;
    pub mod funding_payment_tests;
    pub mod mock_data;
    pub mod integration_tests;
    pub mod performance_tests;
    pub mod regression_tests;
    pub mod trading_mode_tests;
    pub mod unified_data_tests;
    pub mod unified_data_impl_tests;
    pub mod standalone_unified_data_tests;
    pub mod risk_manager_tests;
    pub mod advanced_risk_controls_tests;
    pub mod live_trading_tests;
    pub mod live_trading_safety_tests;
    pub mod real_time_monitoring_tests;
    pub mod trading_strategy_tests;
    // Mode-specific test suites
    pub mod paper_trading_tests;
    pub mod live_trading_integration_tests;
    pub mod strategy_consistency_tests;
    pub mod performance_stress_tests;
    pub mod safety_validation_tests;
    pub mod workflow_tests;
    pub mod standalone_position_tests;
}

// Re-export commonly used types
pub use data::{HyperliquidData, HyperliquidDataFetcher, FundingStatistics};
pub use backtest::{
    HyperliquidBacktest, HyperliquidCommission, OrderType, TradingScenario,
    CommissionTracker, CommissionStats, OrderTypeStrategy
};
pub use strategies::{
    HyperliquidStrategy, TradingSignal, SignalStrength, FundingAwareConfig,
    funding_arbitrage_strategy, enhanced_sma_cross
};
pub use errors::{HyperliquidBacktestError, Result};
pub use indicators::{
    FundingPredictionModel, FundingPredictionConfig, FundingPrediction,
    FundingDirection, FundingVolatility, FundingMomentum, FundingCycle,
    FundingAnomaly, FundingArbitrageOpportunity, FundingPriceCorrelation,
    OpenInterestData, OpenInterestChange, LiquidationData, LiquidationImpact,
    BasisIndicator, FundingRatePredictor
};
pub use funding_report::{
    FundingReport, FundingDistribution, FundingRatePoint,
    FundingDirectionStats, FundingMetricsByPeriod, FundingPeriodMetric
};
pub use backtest::FundingPayment;
pub use csv_export::{
    EnhancedCsvExport, EnhancedCsvExportExt, StrategyComparisonData
};
pub use trading_mode::{
    TradingMode, TradingConfig, RiskConfig, SlippageConfig, ApiConfig,
    TradingModeError
};
pub use trading_mode_impl::{
    TradingModeManager, TradingResult
};
pub use unified_data::{
    Position, OrderRequest, OrderResult, MarketData, Signal, SignalDirection,
    OrderSide, OrderType as TradingOrderType, TimeInForce, OrderStatus,
    TradingStrategy, OrderBookLevel, OrderBookSnapshot, Trade
};
pub use paper_trading::{
    PaperTradingEngine, PaperTradingError, SimulatedOrder, PaperTradingMetrics,
    TradeLogEntry, PaperTradingReport
};
pub use real_time_data_stream::{
    RealTimeDataStream, RealTimeDataError, SubscriptionType, DataSubscription
};
pub use risk_manager::{
    RiskManager, RiskError, RiskOrder, Result as RiskResult
};
pub use live_trading::{
    LiveTradingEngine, LiveTradingError, LiveOrder
};
pub use mode_reporting::{
    ModeReportingManager, CommonPerformanceMetrics, PaperTradingReport as ModeSpecificPaperTradingReport,
    LiveTradingReport, RealTimePnLReport, MonitoringDashboardData, FundingImpactAnalysis,
    RiskMetrics, ConnectionMetrics, AlertEntry, OrderSummary
};
pub use real_time_monitoring::{
    MonitoringServer, MonitoringClient, MonitoringManager, MonitoringError,
    MonitoringMessage, TradeExecutionUpdate, ConnectionStatusUpdate, ConnectionStatus,
    PerformanceMetricsUpdate
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::data::{HyperliquidData, HyperliquidDataFetcher};
    pub use crate::backtest::{
        HyperliquidBacktest, HyperliquidCommission, OrderType, TradingScenario,
        CommissionTracker, CommissionStats, OrderTypeStrategy
    };
    pub use crate::strategies::{
        HyperliquidStrategy, funding_arbitrage_strategy, enhanced_sma_cross
    };
    pub use crate::errors::{HyperliquidBacktestError, Result};
    pub use crate::indicators::{
        FundingDirection, FundingPrediction, FundingRatePredictor,
        OpenInterestData, LiquidationData, BasisIndicator,
        calculate_funding_volatility, calculate_funding_momentum,
        calculate_funding_arbitrage, calculate_basis_indicator
    };
    pub use crate::funding_report::{
        FundingReport, FundingDistribution, FundingRatePoint,
        FundingDirectionStats, FundingMetricsByPeriod, FundingPeriodMetric
    };
    pub use crate::backtest::FundingPayment;
    pub use crate::csv_export::{
        EnhancedCsvExport, EnhancedCsvExportExt, StrategyComparisonData
    };
    pub use crate::logging::{init_logger, init_logger_with_level, performance_span};
    pub use crate::trading_mode::{
        TradingMode, TradingModeError
    };
    pub use crate::trading_mode_impl::{
        TradingModeManager, TradingResult
    };
    pub use crate::unified_data::{
        Position, OrderRequest, OrderResult, MarketData, Signal, SignalDirection,
        OrderSide, TimeInForce, OrderStatus, TradingStrategy,
        OrderBookLevel, OrderBookSnapshot, Trade
    };
    pub use crate::trading_mode::{
        TradingConfig, RiskConfig, SlippageConfig, ApiConfig
    };
    pub use crate::paper_trading::{
        PaperTradingEngine, PaperTradingError, SimulatedOrder, PaperTradingMetrics,
        TradeLogEntry, PaperTradingReport
    };
    pub use crate::real_time_data_stream::{
        RealTimeDataStream, RealTimeDataError, SubscriptionType, DataSubscription
    };
    pub use crate::risk_manager::{
        RiskManager, RiskError, RiskOrder, Result as RiskResult
    };
    pub use crate::live_trading::{
        LiveTradingEngine, LiveTradingError, LiveOrder,
        AlertLevel, AlertMessage, RetryPolicy, SafetyCircuitBreakerConfig
    };
    pub use crate::mode_reporting::{
        ModeReportingManager, CommonPerformanceMetrics, PaperTradingReport as ModeSpecificPaperTradingReport,
        LiveTradingReport, RealTimePnLReport, MonitoringDashboardData, FundingImpactAnalysis,
        RiskMetrics, ConnectionMetrics, AlertEntry, OrderSummary
    };
    pub use crate::real_time_monitoring::{
        MonitoringServer, MonitoringClient, MonitoringManager, MonitoringError,
        MonitoringMessage, TradeExecutionUpdate, ConnectionStatusUpdate, ConnectionStatus,
        PerformanceMetricsUpdate
    };
    pub use chrono::{DateTime, FixedOffset};
}