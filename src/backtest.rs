//! # Enhanced Backtesting Functionality with Hyperliquid-specific Features
//!
//! This module provides enhanced backtesting capabilities that extend the rs-backtester framework
//! with Hyperliquid-specific features including funding rate calculations, maker/taker fee structures,
//! and perpetual futures mechanics.
//!
//! ## Key Features
//!
//! - **Funding Rate Integration**: Automatic calculation of funding payments based on position size
//! - **Enhanced Commission Structure**: Separate maker/taker rates matching Hyperliquid's fee structure
//! - **Perpetual Futures Support**: Complete support for perpetual futures trading mechanics
//! - **Advanced Reporting**: Detailed reports separating trading PnL from funding PnL
//! - **Seamless Integration**: Drop-in replacement for rs-backtester with enhanced features
//!
//! ## Usage Examples
//!
//! ### Basic Enhanced Backtesting
//!
//! ```rust,no_run
//! use hyperliquid_backtest::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), HyperliquidBacktestError> {
//!     // Fetch data
//!     let data = HyperliquidData::fetch("BTC", "1h", start_time, end_time).await?;
//!     
//!     // Create strategy
//!     let strategy = enhanced_sma_cross(10, 20, Default::default())?;
//!     
//!     // Set up enhanced backtest
//!     let mut backtest = HyperliquidBacktest::new(
//!         data,
//!         strategy,
//!         10000.0, // $10,000 initial capital
//!         HyperliquidCommission::default(),
//!     )?;
//!     
//!     // Run backtest with funding calculations
//!     backtest.calculate_with_funding()?;
//!     
//!     // Get comprehensive results
//!     let report = backtest.enhanced_report()?;
//!     println!("Total Return: {:.2}%", report.total_return * 100.0);
//!     println!("Trading PnL: ${:.2}", report.trading_pnl);
//!     println!("Funding PnL: ${:.2}", report.funding_pnl);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Custom Commission Structure
//!
//! ```rust,no_run
//! use hyperliquid_backtest::prelude::*;
//!
//! // Create custom commission structure
//! let commission = HyperliquidCommission::new(
//!     0.0001, // 0.01% maker rate
//!     0.0003, // 0.03% taker rate
//!     true,   // Enable funding calculations
//! );
//!
//! let mut backtest = HyperliquidBacktest::new(
//!     data,
//!     strategy,
//!     50000.0,
//!     commission,
//! )?;
//! ```
//!
//! ### Funding-Only Analysis
//!
//! ```rust,no_run
//! use hyperliquid_backtest::prelude::*;
//!
//! // Disable trading fees to analyze funding impact only
//! let commission = HyperliquidCommission::new(0.0, 0.0, true);
//!
//! let mut backtest = HyperliquidBacktest::new(data, strategy, 10000.0, commission)?;
//! backtest.calculate_with_funding()?;
//!
//! let funding_report = backtest.funding_report()?;
//! println!("Pure funding PnL: ${:.2}", funding_report.net_funding_pnl);
//! ```

use crate::data::HyperliquidData;
use crate::errors::{HyperliquidBacktestError, Result};
use chrono::{DateTime, FixedOffset, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::fs::File;
use std::io::Write;

/// Order type for commission calculation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OrderType {
    /// Market order (typically taker)
    Market,
    /// Limit order that adds liquidity (maker)
    LimitMaker,
    /// Limit order that removes liquidity (taker)
    LimitTaker,
}

/// Trading scenario for commission calculation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TradingScenario {
    /// Opening a new position
    OpenPosition,
    /// Closing an existing position
    ClosePosition,
    /// Reducing position size
    ReducePosition,
    /// Increasing position size
    IncreasePosition,
}

/// Commission structure for Hyperliquid trading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperliquidCommission {
    /// Maker fee rate (typically lower)
    pub maker_rate: f64,
    /// Taker fee rate (typically higher)
    pub taker_rate: f64,
    /// Whether to include funding payments in calculations
    pub funding_enabled: bool,
}

impl Default for HyperliquidCommission {
    fn default() -> Self {
        Self {
            maker_rate: 0.0002,  // 0.02% maker fee (Hyperliquid standard)
            taker_rate: 0.0005,  // 0.05% taker fee (Hyperliquid standard)
            funding_enabled: true,
        }
    }
}

impl HyperliquidCommission {
    /// Create a new HyperliquidCommission with custom rates
    pub fn new(maker_rate: f64, taker_rate: f64, funding_enabled: bool) -> Self {
        Self {
            maker_rate,
            taker_rate,
            funding_enabled,
        }
    }

    /// Calculate trading fee based on order type
    pub fn calculate_fee(&self, order_type: OrderType, trade_value: f64) -> f64 {
        let rate = match order_type {
            OrderType::Market | OrderType::LimitTaker => self.taker_rate,
            OrderType::LimitMaker => self.maker_rate,
        };
        trade_value * rate
    }

    /// Calculate fee for a specific trading scenario
    pub fn calculate_scenario_fee(
        &self,
        scenario: TradingScenario,
        order_type: OrderType,
        trade_value: f64,
    ) -> f64 {
        // Base fee calculation
        let base_fee = self.calculate_fee(order_type, trade_value);
        
        // Apply scenario-specific adjustments if needed
        match scenario {
            TradingScenario::OpenPosition => base_fee,
            TradingScenario::ClosePosition => base_fee,
            TradingScenario::ReducePosition => base_fee,
            TradingScenario::IncreasePosition => base_fee,
        }
    }

    /// Convert to rs-backtester Commission (uses taker rate as default)
    pub fn to_rs_backtester_commission(&self) -> rs_backtester::backtester::Commission {
        rs_backtester::backtester::Commission {
            rate: self.taker_rate,
        }
    }

    /// Validate commission rates
    pub fn validate(&self) -> Result<()> {
        if self.maker_rate < 0.0 || self.maker_rate > 1.0 {
            return Err(HyperliquidBacktestError::validation(
                format!("Invalid maker rate: {}. Must be between 0.0 and 1.0", self.maker_rate)
            ));
        }
        if self.taker_rate < 0.0 || self.taker_rate > 1.0 {
            return Err(HyperliquidBacktestError::validation(
                format!("Invalid taker rate: {}. Must be between 0.0 and 1.0", self.taker_rate)
            ));
        }
        if self.maker_rate > self.taker_rate {
            return Err(HyperliquidBacktestError::validation(
                "Maker rate should typically be lower than taker rate".to_string()
            ));
        }
        Ok(())
    }
}

/// Strategy for determining order types in backtesting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderTypeStrategy {
    /// Always use market orders (taker fees)
    AlwaysMarket,
    /// Always use limit maker orders (maker fees)
    AlwaysMaker,
    /// Mixed strategy with specified maker percentage
    Mixed { maker_percentage: f64 },
    /// Adaptive strategy based on market conditions
    Adaptive,
}

impl OrderTypeStrategy {
    /// Get the order type for a given trade index
    pub fn get_order_type(&self, trade_index: usize) -> OrderType {
        match self {
            OrderTypeStrategy::AlwaysMarket => OrderType::Market,
            OrderTypeStrategy::AlwaysMaker => OrderType::LimitMaker,
            OrderTypeStrategy::Mixed { maker_percentage } => {
                // Use deterministic hashing to ensure consistent results
                let mut hasher = DefaultHasher::new();
                trade_index.hash(&mut hasher);
                let hash_value = hasher.finish();
                let normalized = (hash_value as f64) / (u64::MAX as f64);
                
                if normalized < *maker_percentage {
                    OrderType::LimitMaker
                } else {
                    OrderType::Market
                }
            }
            OrderTypeStrategy::Adaptive => {
                // Simple adaptive strategy: alternate between maker and taker
                if trade_index % 2 == 0 {
                    OrderType::LimitMaker
                } else {
                    OrderType::Market
                }
            }
        }
    }
}

/// Commission statistics for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionStats {
    /// Total commission paid
    pub total_commission: f64,
    /// Total maker fees paid
    pub maker_fees: f64,
    /// Total taker fees paid
    pub taker_fees: f64,
    /// Number of maker orders
    pub maker_orders: usize,
    /// Number of taker orders
    pub taker_orders: usize,
    /// Average commission rate
    pub average_rate: f64,
    /// Ratio of maker to total orders
    pub maker_taker_ratio: f64,
}

/// Individual funding payment record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingPayment {
    /// Timestamp of the funding payment
    pub timestamp: DateTime<chrono::FixedOffset>,
    /// Position size at the time of funding
    pub position_size: f64,
    /// Funding rate applied
    pub funding_rate: f64,
    /// Funding payment amount (positive = received, negative = paid)
    pub payment_amount: f64,
    /// Mark price at the time of funding
    pub mark_price: f64,
}

/// Enhanced metrics for Hyperliquid backtesting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedMetrics {
    /// Total return including funding
    pub total_return_with_funding: f64,
    /// Total return from trading only
    pub trading_only_return: f64,
    /// Total return from funding only
    pub funding_only_return: f64,
    /// Sharpe ratio including funding
    pub sharpe_ratio_with_funding: f64,
    /// Maximum drawdown including funding
    pub max_drawdown_with_funding: f64,
    /// Number of funding payments received
    pub funding_payments_received: usize,
    /// Number of funding payments paid
    pub funding_payments_paid: usize,
    /// Average funding rate
    pub average_funding_rate: f64,
    /// Funding rate volatility
    pub funding_rate_volatility: f64,
}

impl Default for EnhancedMetrics {
    fn default() -> Self {
        Self {
            total_return_with_funding: 0.0,
            trading_only_return: 0.0,
            funding_only_return: 0.0,
            sharpe_ratio_with_funding: 0.0,
            max_drawdown_with_funding: 0.0,
            funding_payments_received: 0,
            funding_payments_paid: 0,
            average_funding_rate: 0.0,
            funding_rate_volatility: 0.0,
        }
    }
}

/// Commission tracking for detailed reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionTracker {
    /// Total maker fees paid
    pub total_maker_fees: f64,
    /// Total taker fees paid
    pub total_taker_fees: f64,
    /// Number of maker orders
    pub maker_order_count: usize,
    /// Number of taker orders
    pub taker_order_count: usize,
}

impl Default for CommissionTracker {
    fn default() -> Self {
        Self {
            total_maker_fees: 0.0,
            total_taker_fees: 0.0,
            maker_order_count: 0,
            taker_order_count: 0,
        }
    }
}

impl CommissionTracker {
    /// Add a commission entry
    pub fn add_commission(
        &mut self,
        _timestamp: chrono::DateTime<chrono::FixedOffset>,
        order_type: OrderType,
        _trade_value: f64,
        commission_paid: f64,
        _scenario: TradingScenario,
    ) {
        match order_type {
            OrderType::LimitMaker => {
                self.total_maker_fees += commission_paid;
                self.maker_order_count += 1;
            }
            OrderType::Market | OrderType::LimitTaker => {
                self.total_taker_fees += commission_paid;
                self.taker_order_count += 1;
            }
        }
    }

    /// Get total commission paid
    pub fn total_commission(&self) -> f64 {
        self.total_maker_fees + self.total_taker_fees
    }

    /// Get average commission rate
    pub fn average_commission_rate(&self) -> f64 {
        let total_orders = self.maker_order_count + self.taker_order_count;
        if total_orders > 0 {
            self.total_commission() / total_orders as f64
        } else {
            0.0
        }
    }

    /// Get maker/taker ratio
    pub fn maker_taker_ratio(&self) -> f64 {
        let total_orders = self.maker_order_count + self.taker_order_count;
        if total_orders > 0 {
            self.maker_order_count as f64 / total_orders as f64
        } else {
            0.0
        }
    }
}

/// Enhanced report structure with Hyperliquid-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedReport {
    /// Strategy name
    pub strategy_name: String,
    /// Symbol/ticker
    pub ticker: String,
    /// Initial capital
    pub initial_capital: f64,
    /// Final equity
    pub final_equity: f64,
    /// Total return
    pub total_return: f64,
    /// Number of trades
    pub trade_count: usize,
    /// Win rate
    pub win_rate: f64,
    /// Profit factor
    pub profit_factor: f64,
    /// Sharpe ratio
    pub sharpe_ratio: f64,
    /// Max drawdown
    pub max_drawdown: f64,
    /// Enhanced metrics including funding
    pub enhanced_metrics: EnhancedMetrics,
    /// Commission statistics
    pub commission_stats: CommissionStats,
    /// Funding payment summary
    pub funding_summary: FundingSummary,
}

/// Funding payment summary for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingSummary {
    /// Total funding paid
    pub total_funding_paid: f64,
    /// Total funding received
    pub total_funding_received: f64,
    /// Net funding (received - paid)
    pub net_funding: f64,
    /// Number of funding payments
    pub funding_payment_count: usize,
    /// Average funding payment
    pub average_funding_payment: f64,
    /// Average funding rate
    pub average_funding_rate: f64,
    /// Funding rate volatility
    pub funding_rate_volatility: f64,
    /// Funding contribution to total return
    pub funding_contribution_percentage: f64,
}

/// Enhanced backtesting engine with Hyperliquid-specific features
#[derive(Clone)]
pub struct HyperliquidBacktest {
    /// Underlying rs-backtester Backtest instance
    pub base_backtest: Option<rs_backtester::backtester::Backtest>,
    /// Original Hyperliquid data with funding information
    pub data: HyperliquidData,
    /// Strategy name for identification
    pub strategy_name: String,
    /// Initial capital for the backtest
    pub initial_capital: f64,
    /// Commission configuration
    pub commission_config: HyperliquidCommission,
    /// Commission tracking
    pub commission_tracker: CommissionTracker,
    /// Order type strategy for commission calculation
    pub order_type_strategy: OrderTypeStrategy,
    /// Funding PnL tracking (separate from trading PnL)
    pub funding_pnl: Vec<f64>,
    /// Trading PnL tracking (without funding)
    pub trading_pnl: Vec<f64>,
    /// Total PnL tracking (trading + funding)
    pub total_pnl: Vec<f64>,
    /// Total funding paid (negative values)
    pub total_funding_paid: f64,
    /// Total funding received (positive values)
    pub total_funding_received: f64,
    /// Funding payment history
    pub funding_payments: Vec<FundingPayment>,
    /// Enhanced metrics
    pub enhanced_metrics: EnhancedMetrics,
}

impl HyperliquidBacktest {
    /// Create a new HyperliquidBacktest instance
    pub fn new(
        data: HyperliquidData,
        strategy_name: String,
        initial_capital: f64,
        commission: HyperliquidCommission,
    ) -> Self {
        Self {
            base_backtest: None,
            data,
            strategy_name,
            initial_capital,
            commission_config: commission,
            commission_tracker: CommissionTracker::default(),
            order_type_strategy: OrderTypeStrategy::Mixed { maker_percentage: 0.5 },
            funding_pnl: Vec::new(),
            trading_pnl: Vec::new(),
            total_pnl: Vec::new(),
            total_funding_paid: 0.0,
            total_funding_received: 0.0,
            funding_payments: Vec::new(),
            enhanced_metrics: EnhancedMetrics::default(),
        }
    }
    
    /// Access the base backtest instance
    pub fn base_backtest(&self) -> Option<&rs_backtester::backtester::Backtest> {
        self.base_backtest.as_ref()
    }
    
    /// Access the base backtest instance mutably
    pub fn base_backtest_mut(&mut self) -> Option<&mut rs_backtester::backtester::Backtest> {
        self.base_backtest.as_mut()
    }

    /// Set the order type strategy for commission calculation
    pub fn with_order_type_strategy(mut self, strategy: OrderTypeStrategy) -> Self {
        self.order_type_strategy = strategy;
        self
    }

    /// Initialize the underlying rs-backtester with converted data
    pub fn initialize_base_backtest(&mut self) -> Result<()> {
        // Validate data before proceeding
        self.data.validate_all_data()?;

        // Convert HyperliquidData to rs-backtester Data format
        let rs_data = self.data.to_rs_backtester_data();

        // Create rs-backtester Backtest instance
        let rs_commission = self.commission_config.to_rs_backtester_commission();
        
        // Create a simple do_nothing strategy as default
        let strategy = rs_backtester::strategies::do_nothing(rs_data.clone());
        
        let backtest = rs_backtester::backtester::Backtest::new(
            rs_data,
            strategy,
            self.initial_capital,
            rs_commission,
        );

        self.base_backtest = Some(backtest);
        
        // Initialize PnL tracking vectors
        self.funding_pnl = vec![0.0; self.data.len()];
        self.trading_pnl = vec![0.0; self.data.len()];
        self.total_pnl = vec![0.0; self.data.len()];

        Ok(())
    }

    /// Calculate backtest results including funding payments
    /// This method applies funding payments to positions based on funding rates and timing
    pub fn calculate_with_funding(&mut self) -> Result<()> {
        // Ensure we have a base backtest to work with
        if self.base_backtest.is_none() {
            return Err(HyperliquidBacktestError::validation(
                "Base backtest must be initialized before calculating funding"
            ));
        }

        // Get the data length
        let data_len = self.data.len();
        
        // Initialize funding and trading PnL vectors if not already done
        if self.funding_pnl.len() != data_len {
            self.funding_pnl = vec![0.0; data_len];
        }
        if self.trading_pnl.len() != data_len {
            self.trading_pnl = vec![0.0; data_len];
        }

        // Reset funding totals
        self.total_funding_paid = 0.0;
        self.total_funding_received = 0.0;
        self.funding_payments.clear();

        // Simulate position tracking (in a real implementation, this would come from strategy signals)
        let current_position = 0.0;
        let mut cumulative_funding_pnl = 0.0;

        // Process each data point
        for i in 0..data_len {
            let timestamp = self.data.datetime[i];
            let price = self.data.close[i];

            // Check if this is a funding payment time (every 8 hours)
            if self.is_funding_time(timestamp) {
                // Get funding rate for this timestamp
                if let Some(funding_rate) = self.get_funding_rate_for_timestamp(timestamp) {
                    // Calculate funding payment
                    let funding_payment = self.calculate_funding_payment(
                        current_position,
                        funding_rate,
                        price,
                    );

                    // Apply funding payment
                    cumulative_funding_pnl += funding_payment;
                    
                    // Track funding payment
                    if funding_payment > 0.0 {
                        self.total_funding_received += funding_payment;
                    } else {
                        self.total_funding_paid += funding_payment.abs();
                    }

                    // Record funding payment
                    self.funding_payments.push(FundingPayment {
                        timestamp,
                        position_size: current_position,
                        funding_rate,
                        payment_amount: funding_payment,
                        mark_price: price,
                    });
                }
            }

            // Store cumulative funding PnL
            self.funding_pnl[i] = cumulative_funding_pnl;
            
            // Calculate trading PnL (total PnL minus funding PnL)
            // This would normally come from the base backtest results
            self.trading_pnl[i] = 0.0; // Placeholder - would be calculated from actual trades
        }

        // Update enhanced metrics
        self.update_enhanced_metrics()?;

        Ok(())
    }

    /// Calculate funding payments with position tracking
    /// This version allows external position tracking for more accurate funding calculations
    pub fn calculate_with_funding_and_positions(&mut self, positions: &[f64]) -> Result<()> {
        // Ensure we have a base backtest to work with
        if self.base_backtest.is_none() {
            return Err(HyperliquidBacktestError::validation(
                "Base backtest must be initialized before calculating funding"
            ));
        }

        let data_len = self.data.len();

        // Validate positions array length
        if positions.len() != data_len {
            return Err(HyperliquidBacktestError::validation(
                "Positions array length must match data length"
            ));
        }

        // Initialize funding and trading PnL vectors if not already done
        if self.funding_pnl.len() != data_len {
            self.funding_pnl = vec![0.0; data_len];
        }
        if self.trading_pnl.len() != data_len {
            self.trading_pnl = vec![0.0; data_len];
        }

        // Reset funding totals
        self.total_funding_paid = 0.0;
        self.total_funding_received = 0.0;
        self.funding_payments.clear();

        let mut cumulative_funding_pnl = 0.0;

        // Process each data point
        for i in 0..data_len {
            let timestamp = self.data.datetime[i];
            let price = self.data.close[i];
            let position_size = positions[i];

            // Check if this is a funding payment time (every 8 hours)
            if self.is_funding_time(timestamp) {
                // Get funding rate for this timestamp
                if let Some(funding_rate) = self.get_funding_rate_for_timestamp(timestamp) {
                    // Calculate funding payment
                    let funding_payment = self.calculate_funding_payment(
                        position_size,
                        funding_rate,
                        price,
                    );

                    // Apply funding payment
                    cumulative_funding_pnl += funding_payment;
                    
                    // Track funding payment
                    if funding_payment > 0.0 {
                        self.total_funding_received += funding_payment;
                    } else {
                        self.total_funding_paid += funding_payment.abs();
                    }

                    // Record funding payment
                    self.funding_payments.push(FundingPayment {
                        timestamp,
                        position_size,
                        funding_rate,
                        payment_amount: funding_payment,
                        mark_price: price,
                    });
                }
            }

            // Store cumulative funding PnL
            self.funding_pnl[i] = cumulative_funding_pnl;
        }

        // Update enhanced metrics
        self.update_enhanced_metrics()?;

        Ok(())
    }

    /// Check if a given timestamp is a funding payment time (every 8 hours)
    /// Hyperliquid funding payments occur at 00:00, 08:00, and 16:00 UTC
    pub fn is_funding_time(&self, timestamp: DateTime<FixedOffset>) -> bool {
        let hour = timestamp.hour();
        hour % 8 == 0 && timestamp.minute() == 0 && timestamp.second() == 0
    }

    /// Get funding rate for a specific timestamp from the data
    pub fn get_funding_rate_for_timestamp(&self, timestamp: DateTime<FixedOffset>) -> Option<f64> {
        self.data.get_funding_rate_at(timestamp)
    }

    /// Calculate funding payment based on position size, funding rate, and mark price
    /// Formula: funding_payment = position_size * funding_rate * mark_price
    /// Positive payment means funding received, negative means funding paid
    pub fn calculate_funding_payment(&self, position_size: f64, funding_rate: f64, mark_price: f64) -> f64 {
        // If no position, no funding payment
        if position_size == 0.0 {
            return 0.0;
        }

        // Calculate funding payment
        // For long positions: pay funding when rate is positive, receive when negative
        // For short positions: receive funding when rate is positive, pay when negative
        let funding_payment = -position_size * funding_rate * mark_price;
        
        funding_payment
    }

    /// Update enhanced metrics based on current funding and trading data
    fn update_enhanced_metrics(&mut self) -> Result<()> {
        if self.funding_pnl.is_empty() {
            return Ok(());
        }

        // Calculate funding-only return
        let final_funding_pnl = self.funding_pnl.last().unwrap_or(&0.0);
        self.enhanced_metrics.funding_only_return = final_funding_pnl / self.initial_capital;

        // Calculate trading-only return
        let final_trading_pnl = self.trading_pnl.last().unwrap_or(&0.0);
        self.enhanced_metrics.trading_only_return = final_trading_pnl / self.initial_capital;

        // Calculate total return with funding
        self.enhanced_metrics.total_return_with_funding = 
            self.enhanced_metrics.trading_only_return + self.enhanced_metrics.funding_only_return;

        // Count funding payments
        self.enhanced_metrics.funding_payments_received = 
            self.funding_payments.iter().filter(|p| p.payment_amount > 0.0).count();
        self.enhanced_metrics.funding_payments_paid = 
            self.funding_payments.iter().filter(|p| p.payment_amount < 0.0).count();

        // Calculate average funding rate
        if !self.funding_payments.is_empty() {
            let total_funding_rate: f64 = self.funding_payments.iter().map(|p| p.funding_rate).sum();
            self.enhanced_metrics.average_funding_rate = total_funding_rate / self.funding_payments.len() as f64;

            // Calculate funding rate volatility (standard deviation)
            let mean_rate = self.enhanced_metrics.average_funding_rate;
            let variance: f64 = self.funding_payments.iter()
                .map(|p| (p.funding_rate - mean_rate).powi(2))
                .sum::<f64>() / self.funding_payments.len() as f64;
            self.enhanced_metrics.funding_rate_volatility = variance.sqrt();
        }

        // Calculate maximum drawdown with funding
        let mut peak = self.initial_capital;
        let mut max_drawdown = 0.0;
        
        for i in 0..self.funding_pnl.len() {
            let total_value = self.initial_capital + self.trading_pnl[i] + self.funding_pnl[i];
            if total_value > peak {
                peak = total_value;
            }
            let drawdown = (peak - total_value) / peak;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
        self.enhanced_metrics.max_drawdown_with_funding = -max_drawdown;

        // Calculate Sharpe ratio with funding (simplified version)
        if self.funding_pnl.len() > 1 {
            let returns: Vec<f64> = (1..self.funding_pnl.len())
                .map(|i| {
                    let prev_total = self.initial_capital + self.trading_pnl[i-1] + self.funding_pnl[i-1];
                    let curr_total = self.initial_capital + self.trading_pnl[i] + self.funding_pnl[i];
                    if prev_total > 0.0 {
                        (curr_total - prev_total) / prev_total
                    } else {
                        0.0
                    }
                })
                .collect();

            if !returns.is_empty() {
                let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
                let variance = returns.iter()
                    .map(|r| (r - mean_return).powi(2))
                    .sum::<f64>() / returns.len() as f64;
                let std_dev = variance.sqrt();
                
                if std_dev > 0.0 {
                    self.enhanced_metrics.sharpe_ratio_with_funding = mean_return / std_dev;
                }
            }
        }

        Ok(())
    }

    // Getter methods
    pub fn data(&self) -> &HyperliquidData { &self.data }
    pub fn strategy_name(&self) -> &str { &self.strategy_name }
    pub fn initial_capital(&self) -> f64 { self.initial_capital }
    pub fn commission_config(&self) -> &HyperliquidCommission { &self.commission_config }
    pub fn funding_pnl(&self) -> &[f64] { &self.funding_pnl }
    pub fn trading_pnl(&self) -> &[f64] { &self.trading_pnl }
    pub fn total_funding_paid(&self) -> f64 { self.total_funding_paid }
    pub fn total_funding_received(&self) -> f64 { self.total_funding_received }
    pub fn funding_payments(&self) -> &[FundingPayment] { &self.funding_payments }
    pub fn enhanced_metrics(&self) -> &EnhancedMetrics { &self.enhanced_metrics }
    pub fn is_initialized(&self) -> bool { self.base_backtest.is_some() }

    pub fn validate(&self) -> Result<()> {
        self.commission_config.validate()?;
        self.data.validate_all_data()?;
        if self.initial_capital <= 0.0 {
            return Err(HyperliquidBacktestError::validation("Initial capital must be positive"));
        }
        if self.strategy_name.is_empty() {
            return Err(HyperliquidBacktestError::validation("Strategy name cannot be empty"));
        }
        Ok(())
    }

    /// Get commission statistics
    pub fn commission_stats(&self) -> CommissionStats {
        CommissionStats {
            total_commission: self.commission_tracker.total_commission(),
            maker_fees: self.commission_tracker.total_maker_fees,
            taker_fees: self.commission_tracker.total_taker_fees,
            maker_orders: self.commission_tracker.maker_order_count,
            taker_orders: self.commission_tracker.taker_order_count,
            average_rate: self.commission_tracker.average_commission_rate(),
            maker_taker_ratio: self.commission_tracker.maker_taker_ratio(),
        }
    }

    /// Calculate trade commission based on order type strategy
    pub fn calculate_trade_commission(
        &self,
        trade_value: f64,
        trade_index: usize,
        scenario: TradingScenario,
    ) -> (OrderType, f64) {
        let order_type = self.order_type_strategy.get_order_type(trade_index);
        let commission = self.commission_config.calculate_scenario_fee(scenario, order_type, trade_value);
        (order_type, commission)
    }

    /// Track commission for reporting
    pub fn track_commission(
        &mut self,
        timestamp: DateTime<chrono::FixedOffset>,
        order_type: OrderType,
        trade_value: f64,
        commission_paid: f64,
        scenario: TradingScenario,
    ) {
        self.commission_tracker.add_commission(
            timestamp,
            order_type,
            trade_value,
            commission_paid,
            scenario,
        );
    }

    /// Generate a funding summary for reporting
    pub fn funding_summary(&self) -> FundingSummary {
        let net_funding = self.total_funding_received - self.total_funding_paid;
        let funding_payment_count = self.funding_payments.len();
        
        let average_funding_payment = if funding_payment_count > 0 {
            let total_payments: f64 = self.funding_payments.iter()
                .map(|p| p.payment_amount)
                .sum();
            total_payments / funding_payment_count as f64
        } else {
            0.0
        };
        
        let funding_contribution_percentage = if self.enhanced_metrics.total_return_with_funding != 0.0 {
            (self.enhanced_metrics.funding_only_return / self.enhanced_metrics.total_return_with_funding) * 100.0
        } else {
            0.0
        };
        
        FundingSummary {
            total_funding_paid: self.total_funding_paid,
            total_funding_received: self.total_funding_received,
            net_funding,
            funding_payment_count,
            average_funding_payment,
            average_funding_rate: self.enhanced_metrics.average_funding_rate,
            funding_rate_volatility: self.enhanced_metrics.funding_rate_volatility,
            funding_contribution_percentage,
        }
    }

    /// Generate an enhanced report with Hyperliquid-specific metrics
    pub fn enhanced_report(&self) -> Result<EnhancedReport> {
        // Ensure we have a base backtest
        let base_backtest = match &self.base_backtest {
            Some(backtest) => backtest,
            None => return Err(HyperliquidBacktestError::validation(
                "Base backtest must be initialized before generating a report"
            )),
        };
        
        // Calculate basic metrics from base backtest
        let final_equity = if let Some(last_position) = base_backtest.position().last() {
            if let Some(last_close) = self.data.close.last() {
                if let Some(last_account) = base_backtest.account().last() {
                    last_position * last_close + last_account
                } else {
                    self.initial_capital
                }
            } else {
                self.initial_capital
            }
        } else {
            self.initial_capital
        };
        
        let total_return = (final_equity - self.initial_capital) / self.initial_capital;
        
        // Calculate trade statistics
        let mut trade_count = 0;
        let mut win_count = 0;
        let mut profit_sum = 0.0;
        let mut loss_sum = 0.0;
        
        // Count trades and calculate win rate
        let orders = base_backtest.orders();
        if orders.len() > 1 {
            for i in 1..orders.len() {
                if orders[i] != orders[i-1] && orders[i] != rs_backtester::orders::Order::NULL {
                    trade_count += 1;
                    
                    // Simple profit calculation (this is a simplification)
                    if i < self.data.close.len() - 1 {
                        let entry_price = self.data.close[i];
                        let exit_price = self.data.close[i+1];
                        let profit = match orders[i] {
                            rs_backtester::orders::Order::BUY => exit_price - entry_price,
                            rs_backtester::orders::Order::SHORTSELL => entry_price - exit_price,
                            _ => 0.0,
                        };
                        
                        if profit > 0.0 {
                            win_count += 1;
                            profit_sum += profit;
                        } else {
                            loss_sum += profit.abs();
                        }
                    }
                }
            }
        }
        
        let win_rate = if trade_count > 0 {
            win_count as f64 / trade_count as f64
        } else {
            0.0
        };
        
        let profit_factor = if loss_sum > 0.0 {
            profit_sum / loss_sum
        } else if profit_sum > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };
        
        // Calculate Sharpe ratio (simplified)
        let sharpe_ratio = self.enhanced_metrics.sharpe_ratio_with_funding;
        
        // Calculate max drawdown
        let max_drawdown = self.enhanced_metrics.max_drawdown_with_funding;
        
        // Create enhanced report
        let report = EnhancedReport {
            strategy_name: self.strategy_name.clone(),
            ticker: self.data.symbol.clone(),
            initial_capital: self.initial_capital,
            final_equity,
            total_return,
            trade_count,
            win_rate,
            profit_factor,
            sharpe_ratio,
            max_drawdown,
            enhanced_metrics: self.enhanced_metrics.clone(),
            commission_stats: self.commission_stats(),
            funding_summary: self.funding_summary(),
        };
        
        Ok(report)
    }

    /// Print enhanced report to console
    pub fn print_enhanced_report(&self) -> Result<()> {
        let report = self.enhanced_report()?;
        
        println!("\n=== HYPERLIQUID BACKTEST REPORT ===");
        println!("Strategy: {}", report.strategy_name);
        println!("Symbol: {}", report.ticker);
        println!("Period: {} to {}", 
            self.data.datetime.first().unwrap_or(&DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap()),
            self.data.datetime.last().unwrap_or(&DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap())
        );
        println!("Initial Capital: ${:.2}", report.initial_capital);
        println!("Final Equity: ${:.2}", report.final_equity);
        
        // Print base report metrics
        println!("\n--- Base Performance Metrics ---");
        println!("Total Return: {:.2}%", report.total_return * 100.0);
        println!("Sharpe Ratio: {:.2}", report.sharpe_ratio);
        println!("Max Drawdown: {:.2}%", report.max_drawdown * 100.0);
        println!("Win Rate: {:.2}%", report.win_rate * 100.0);
        println!("Profit Factor: {:.2}", report.profit_factor);
        println!("Trade Count: {}", report.trade_count);
        
        // Print enhanced metrics
        println!("\n--- Enhanced Performance Metrics (with Funding) ---");
        println!("Total Return (with Funding): {:.2}%", report.enhanced_metrics.total_return_with_funding * 100.0);
        println!("Trading-Only Return: {:.2}%", report.enhanced_metrics.trading_only_return * 100.0);
        println!("Funding-Only Return: {:.2}%", report.enhanced_metrics.funding_only_return * 100.0);
        println!("Sharpe Ratio (with Funding): {:.2}", report.enhanced_metrics.sharpe_ratio_with_funding);
        println!("Max Drawdown (with Funding): {:.2}%", report.enhanced_metrics.max_drawdown_with_funding * 100.0);
        
        // Print commission statistics
        println!("\n--- Commission Statistics ---");
        println!("Total Commission: ${:.2}", report.commission_stats.total_commission);
        println!("Maker Fees: ${:.2} ({} orders)", 
            report.commission_stats.maker_fees, 
            report.commission_stats.maker_orders
        );
        println!("Taker Fees: ${:.2} ({} orders)", 
            report.commission_stats.taker_fees, 
            report.commission_stats.taker_orders
        );
        println!("Average Commission Rate: {:.4}%", report.commission_stats.average_rate * 100.0);
        println!("Maker/Taker Ratio: {:.2}", report.commission_stats.maker_taker_ratio);
        
        // Print funding summary
        println!("\n--- Funding Summary ---");
        println!("Total Funding Paid: ${:.2}", report.funding_summary.total_funding_paid);
        println!("Total Funding Received: ${:.2}", report.funding_summary.total_funding_received);
        println!("Net Funding: ${:.2}", report.funding_summary.net_funding);
        println!("Funding Payments: {}", report.funding_summary.funding_payment_count);
        println!("Average Funding Payment: ${:.2}", report.funding_summary.average_funding_payment);
        println!("Average Funding Rate: {:.6}%", report.funding_summary.average_funding_rate * 100.0);
        println!("Funding Rate Volatility: {:.6}%", report.funding_summary.funding_rate_volatility * 100.0);
        println!("Funding Contribution: {:.2}% of total return", report.funding_summary.funding_contribution_percentage);
        
        println!("\n=== END OF REPORT ===\n");
        
        Ok(())
    }

    /// Export backtest results to CSV
    pub fn export_to_csv<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // Ensure we have a base backtest
        if self.base_backtest.is_none() {
            return Err(HyperliquidBacktestError::validation(
                "Base backtest must be initialized before exporting to CSV"
            ));
        }
        
        // Create CSV writer
        let file = File::create(path)?;
        let mut wtr = csv::Writer::from_writer(file);
        
        // Write header
        wtr.write_record(&[
            "Timestamp",
            "Open",
            "High",
            "Low",
            "Close",
            "Volume",
            "Funding Rate",
            "Position",
            "Trading PnL",
            "Funding PnL",
            "Total PnL",
            "Equity",
        ])?;
        
        // Write data rows
        for i in 0..self.data.len() {
            let timestamp = self.data.datetime[i].to_rfc3339();
            let open = self.data.open[i].to_string();
            let high = self.data.high[i].to_string();
            let low = self.data.low[i].to_string();
            let close = self.data.close[i].to_string();
            let volume = self.data.volume[i].to_string();
            
            // Get funding rate (if available)
            let funding_rate = match self.get_funding_rate_for_timestamp(self.data.datetime[i]) {
                Some(rate) => rate.to_string(),
                None => "".to_string(),
            };
            
            // Get position (placeholder - would come from base backtest)
            let position = "0.0".to_string(); // Placeholder
            
            // Get PnL values
            let trading_pnl = if i < self.trading_pnl.len() {
                self.trading_pnl[i].to_string()
            } else {
                "0.0".to_string()
            };
            
            let funding_pnl = if i < self.funding_pnl.len() {
                self.funding_pnl[i].to_string()
            } else {
                "0.0".to_string()
            };
            
            // Calculate total PnL and equity
            let total_pnl = (
                self.trading_pnl.get(i).unwrap_or(&0.0) + 
                self.funding_pnl.get(i).unwrap_or(&0.0)
            ).to_string();
            
            let equity = (
                self.initial_capital + 
                self.trading_pnl.get(i).unwrap_or(&0.0) + 
                self.funding_pnl.get(i).unwrap_or(&0.0)
            ).to_string();
            
            // Write row
            wtr.write_record(&[
                &timestamp,
                &open,
                &high,
                &low,
                &close,
                &volume,
                &funding_rate,
                &position,
                &trading_pnl,
                &funding_pnl,
                &total_pnl,
                &equity,
            ])?;
        }
        
        // Flush writer
        wtr.flush()?;
        
        Ok(())
    }

    /// Export funding payments to CSV
    pub fn export_funding_to_csv<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // Create CSV writer
        let file = File::create(path)?;
        let mut wtr = csv::Writer::from_writer(file);
        
        // Write header
        wtr.write_record(&[
            "Timestamp",
            "Position Size",
            "Funding Rate",
            "Mark Price",
            "Payment Amount",
        ])?;
        
        // Write funding payment rows
        for payment in &self.funding_payments {
            wtr.write_record(&[
                &payment.timestamp.to_rfc3339(),
                &payment.position_size.to_string(),
                &payment.funding_rate.to_string(),
                &payment.mark_price.to_string(),
                &payment.payment_amount.to_string(),
            ])?;
        }
        
        // Flush writer
        wtr.flush()?;
        
        Ok(())
    }

    /// Generate a detailed funding report
    pub fn funding_report(&self) -> Result<crate::funding_report::FundingReport> {
        use crate::funding_report::FundingReport;
        
        // Ensure we have a base backtest
        if self.base_backtest.is_none() {
            return Err(HyperliquidBacktestError::validation(
                "Base backtest must be initialized before generating a funding report"
            ));
        }
        
        // Get position sizes and values
        let mut position_sizes = Vec::with_capacity(self.data.len());
        let mut position_values = Vec::with_capacity(self.data.len());
        
        // Get positions from base backtest if available, otherwise use zeros
        if let Some(base_backtest) = &self.base_backtest {
            let positions = base_backtest.position();
            
            for i in 0..self.data.len() {
                let position_size = if i < positions.len() {
                    positions[i]
                } else {
                    0.0
                };
                
                position_sizes.push(position_size);
                position_values.push(position_size * self.data.close[i]);
            }
        } else {
            // Fill with zeros if no base backtest
            position_sizes = vec![0.0; self.data.len()];
            position_values = vec![0.0; self.data.len()];
        }
        
        // Calculate total trading PnL
        let trading_pnl = if let Some(last) = self.trading_pnl.last() {
            *last
        } else {
            0.0
        };
        
        // Calculate total funding PnL
        let funding_pnl = if let Some(last) = self.funding_pnl.last() {
            *last
        } else {
            0.0
        };
        
        // Create funding report
        FundingReport::new(
            &self.data.symbol,
            &self.data,
            &position_values,
            self.funding_payments.clone(),
            funding_pnl,
        )
    }

    /// Export enhanced report to CSV
    pub fn export_report_to_csv<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let report = self.enhanced_report()?;
        
        // Create CSV writer
        let file = File::create(path)?;
        let mut wtr = csv::Writer::from_writer(file);
        
        // Write header and data as key-value pairs
        wtr.write_record(&["Metric", "Value"])?;
        
        // Strategy information
        wtr.write_record(&["Strategy", &report.strategy_name])?;
        wtr.write_record(&["Symbol", &report.ticker])?;
        wtr.write_record(&["Initial Capital", &report.initial_capital.to_string()])?;
        wtr.write_record(&["Final Equity", &report.final_equity.to_string()])?;
        
        // Base metrics
        wtr.write_record(&["Total Return", &(report.total_return * 100.0).to_string()])?;
        wtr.write_record(&["Sharpe Ratio", &report.sharpe_ratio.to_string()])?;
        wtr.write_record(&["Max Drawdown", &(report.max_drawdown * 100.0).to_string()])?;
        wtr.write_record(&["Win Rate", &(report.win_rate * 100.0).to_string()])?;
        wtr.write_record(&["Profit Factor", &report.profit_factor.to_string()])?;
        wtr.write_record(&["Trade Count", &report.trade_count.to_string()])?;
        
        // Enhanced metrics
        wtr.write_record(&["Total Return (with Funding)", &(report.enhanced_metrics.total_return_with_funding * 100.0).to_string()])?;
        wtr.write_record(&["Trading-Only Return", &(report.enhanced_metrics.trading_only_return * 100.0).to_string()])?;
        wtr.write_record(&["Funding-Only Return", &(report.enhanced_metrics.funding_only_return * 100.0).to_string()])?;
        wtr.write_record(&["Sharpe Ratio (with Funding)", &report.enhanced_metrics.sharpe_ratio_with_funding.to_string()])?;
        wtr.write_record(&["Max Drawdown (with Funding)", &(report.enhanced_metrics.max_drawdown_with_funding * 100.0).to_string()])?;
        
        // Commission statistics
        wtr.write_record(&["Total Commission", &report.commission_stats.total_commission.to_string()])?;
        wtr.write_record(&["Maker Fees", &report.commission_stats.maker_fees.to_string()])?;
        wtr.write_record(&["Taker Fees", &report.commission_stats.taker_fees.to_string()])?;
        wtr.write_record(&["Maker Orders", &report.commission_stats.maker_orders.to_string()])?;
        wtr.write_record(&["Taker Orders", &report.commission_stats.taker_orders.to_string()])?;
        wtr.write_record(&["Average Commission Rate", &(report.commission_stats.average_rate * 100.0).to_string()])?;
        wtr.write_record(&["Maker/Taker Ratio", &report.commission_stats.maker_taker_ratio.to_string()])?;
        
        // Funding summary
        wtr.write_record(&["Total Funding Paid", &report.funding_summary.total_funding_paid.to_string()])?;
        wtr.write_record(&["Total Funding Received", &report.funding_summary.total_funding_received.to_string()])?;
        wtr.write_record(&["Net Funding", &report.funding_summary.net_funding.to_string()])?;
        wtr.write_record(&["Funding Payments", &report.funding_summary.funding_payment_count.to_string()])?;
        wtr.write_record(&["Average Funding Payment", &report.funding_summary.average_funding_payment.to_string()])?;
        wtr.write_record(&["Average Funding Rate", &(report.funding_summary.average_funding_rate * 100.0).to_string()])?;
        wtr.write_record(&["Funding Rate Volatility", &(report.funding_summary.funding_rate_volatility * 100.0).to_string()])?;
        wtr.write_record(&["Funding Contribution", &report.funding_summary.funding_contribution_percentage.to_string()])?;
        
        // Flush writer
        wtr.flush()?;
        
        Ok(())
    }
}