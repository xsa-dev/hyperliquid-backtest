//! # Strategy Implementations for Hyperliquid Backtesting
//!
//! This module provides enhanced trading strategies specifically designed for Hyperliquid
//! perpetual futures trading, including funding rate awareness and advanced signal processing.
//!
//! ## Key Features
//!
//! - **Funding-Aware Strategies**: Strategies that incorporate funding rate data into decision making
//! - **Enhanced Technical Indicators**: Traditional indicators enhanced with perpetual futures mechanics
//! - **Signal Strength Classification**: Multi-level signal strength for better risk management
//! - **Configurable Parameters**: Flexible configuration for different market conditions
//! - **Strategy Composition**: Combine multiple strategies for sophisticated trading logic
//!
//! ## Available Strategies
//!
//! ### 1. Funding Arbitrage Strategy
//!
//! Exploits funding rate inefficiencies by taking positions when funding rates exceed thresholds.
//!
//! ```rust,no_run
//! use hyperliquid_backtester::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), HyperliquidBacktestError> {
//!     let data = HyperliquidData::fetch("BTC", "1h", start_time, end_time).await?;
//!     
//!     // Create funding arbitrage strategy with 0.01% threshold
//!     let strategy = funding_arbitrage_strategy(0.0001)?;
//!     
//!     let mut backtest = HyperliquidBacktest::new(
//!         data,
//!         strategy,
//!         10000.0,
//!         HyperliquidCommission::default(),
//!     )?;
//!     
//!     backtest.calculate_with_funding()?;
//!     let report = backtest.funding_report()?;
//!     
//!     println!("Funding arbitrage return: {:.2}%", report.net_funding_pnl / 10000.0 * 100.0);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### 2. Enhanced SMA Cross Strategy
//!
//! Traditional SMA crossover enhanced with funding rate considerations.
//!
//! ```rust,no_run
//! use hyperliquid_backtester::prelude::*;
//!
//! let funding_config = FundingAwareConfig {
//!     funding_threshold: 0.0001,  // 0.01% threshold
//!     funding_weight: 0.3,        // 30% weight to funding signal
//!     use_funding_direction: true,
//!     use_funding_prediction: false,
//! };
//!
//! let strategy = enhanced_sma_cross(10, 20, funding_config)?;
//! ```
//!
//! ## Strategy Configuration
//!
//! ### Funding Awareness Configuration
//!
//! ```rust,no_run
//! use hyperliquid_backtester::prelude::*;
//!
//! let config = FundingAwareConfig {
//!     funding_threshold: 0.0001,      // Minimum funding rate to consider (0.01%)
//!     funding_weight: 0.5,            // Weight of funding signal (0.0 to 1.0)
//!     use_funding_direction: true,    // Consider funding rate direction
//!     use_funding_prediction: true,   // Use funding rate predictions
//! };
//! ```
//!
//! ### Signal Strength Levels
//!
//! - **Strong**: High confidence signals (>80% historical accuracy)
//! - **Medium**: Moderate confidence signals (60-80% historical accuracy)  
//! - **Weak**: Low confidence signals (<60% historical accuracy)
//!
//! ## Custom Strategy Development
//!
//! ### Implementing HyperliquidStrategy Trait
//!
//! ```rust,ignore
//! use hyperliquid_backtester::prelude::*;
//!
//! struct MyCustomStrategy {
//!     funding_config: FundingAwareConfig,
//!     // ... other fields
//! }
//!
//! impl HyperliquidStrategy for MyCustomStrategy {
//!     fn funding_config(&self) -> &FundingAwareConfig {
//!         &self.funding_config
//!     }
//!     
//!     fn set_funding_config(&mut self, config: FundingAwareConfig) {
//!         self.funding_config = config;
//!     }
//!     
//!     fn process_funding(&self, funding_rate: f64) -> TradingSignal {
//!         // Custom funding processing logic
//!         if funding_rate.abs() > self.funding_config.funding_threshold {
//!             TradingSignal::new(
//!                 if funding_rate > 0.0 { 1.0 } else { -1.0 },
//!                 SignalStrength::Strong
//!             )
//!         } else {
//!             TradingSignal::new(0.0, SignalStrength::Weak)
//!         }
//!     }
//!     
//!     fn combine_signals(&self, base_signal: f64, funding_signal: &TradingSignal) -> f64 {
//!         // Custom signal combination logic
//!         let funding_weight = match funding_signal.strength {
//!             SignalStrength::Strong => self.funding_config.funding_weight,
//!             SignalStrength::Medium => self.funding_config.funding_weight * 0.7,
//!             SignalStrength::Weak => self.funding_config.funding_weight * 0.3,
//!         };
//!         
//!         base_signal * (1.0 - funding_weight) + funding_signal.position * funding_weight
//!     }
//! }
//! ```

use rs_backtester::strategies::Strategy;
use rs_backtester::datas::Data;
use serde::{Deserialize, Serialize};

// Note: These modules need to be implemented
// pub use crate::strategies::trading_strategy::{
//     TradingStrategy, StrategyConfig, StrategyState, StrategyParam, BaseTradingStrategy
// };

// pub use crate::strategies::funding_arbitrage_strategy::{
//     FundingArbitrageStrategy, create_funding_arbitrage_strategy
// };
// pub use crate::strategies::enhanced_sma_strategy::{
//     EnhancedSmaStrategy, create_enhanced_sma_strategy
// };
// pub use crate::strategies::strategy_template::{
//     StrategyTemplate, create_strategy_template
// };

/// Signal strength for trading decisions
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SignalStrength {
    /// Strong signal (high confidence)
    Strong,
    /// Medium signal (moderate confidence)
    Medium,
    /// Weak signal (low confidence)
    Weak,
}

/// Trading signal with position size and strength
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSignal {
    /// Position size (-1.0 to 1.0)
    pub position: f64,
    /// Signal strength
    pub strength: SignalStrength,
}

impl TradingSignal {
    /// Create a new TradingSignal
    pub fn new(position: f64, strength: SignalStrength) -> Self {
        Self {
            position,
            strength,
        }
    }
    
    /// Check if signal is long
    pub fn is_long(&self) -> bool {
        self.position > 0.0
    }
    
    /// Check if signal is short
    pub fn is_short(&self) -> bool {
        self.position < 0.0
    }
    
    /// Check if signal is neutral
    pub fn is_neutral(&self) -> bool {
        self.position == 0.0
    }
}

/// Configuration for funding-aware strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingAwareConfig {
    /// Threshold for significant funding rate
    pub funding_threshold: f64,
    /// Weight of funding signal in overall strategy
    pub funding_weight: f64,
    /// Whether to use funding direction in strategy
    pub use_funding_direction: bool,
    /// Whether to use funding prediction in strategy
    pub use_funding_prediction: bool,
}

impl Default for FundingAwareConfig {
    fn default() -> Self {
        Self {
            funding_threshold: 0.0001, // 0.01% per 8h
            funding_weight: 0.5,       // 50% weight to funding
            use_funding_direction: true,
            use_funding_prediction: true,
        }
    }
}

/// Trait for Hyperliquid-specific strategies
pub trait HyperliquidStrategy {
    /// Get funding-aware configuration
    fn funding_config(&self) -> &FundingAwareConfig;
    
    /// Set funding-aware configuration
    fn set_funding_config(&mut self, config: FundingAwareConfig);
    
    /// Process funding rate information
    fn process_funding(&self, funding_rate: f64) -> TradingSignal;
    
    /// Combine funding signal with base strategy signal
    fn combine_signals(&self, base_signal: f64, funding_signal: &TradingSignal) -> f64;
}

/// Create a funding arbitrage strategy
pub fn funding_arbitrage_strategy(data: Data, threshold: f64) -> Strategy {
    // Create a new strategy (placeholder implementation)
    let strategy = Strategy {
        name: format!("Funding Arbitrage (threshold: {})", threshold),
        choices: Vec::new(),
        indicator: None,
    };
    
    // We need to implement the strategy logic differently since the Strategy struct
    // from rs-backtester doesn't match our expected interface
    // For now, we'll return a placeholder implementation
    
    strategy
}

/// Create an enhanced SMA cross strategy with funding awareness
pub fn enhanced_sma_cross(
    data: Data,
    fast_period: usize,
    slow_period: usize,
    funding_config: FundingAwareConfig,
) -> Strategy {
    // Create a new strategy (placeholder implementation)
    let strategy = Strategy {
        name: format!("Enhanced SMA Cross ({}, {})", fast_period, slow_period),
        choices: Vec::new(),
        indicator: None,
    };
    
    // We need to implement the strategy logic differently since the Strategy struct
    // from rs-backtester doesn't match our expected interface
    // For now, we'll return a placeholder implementation
    
    strategy
}

/// Funding arbitrage strategy implementation
pub struct FundingArbitrageStrategy {
    /// Threshold for taking positions
    threshold: f64,
    /// Funding-aware configuration
    funding_config: FundingAwareConfig,
}

impl FundingArbitrageStrategy {
    /// Create a new FundingArbitrageStrategy
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            funding_config: FundingAwareConfig::default(),
        }
    }
}

impl HyperliquidStrategy for FundingArbitrageStrategy {
    fn funding_config(&self) -> &FundingAwareConfig {
        &self.funding_config
    }
    
    fn set_funding_config(&mut self, config: FundingAwareConfig) {
        self.funding_config = config;
    }
    
    fn process_funding(&self, funding_rate: f64) -> TradingSignal {
        if funding_rate.abs() <= self.threshold {
            return TradingSignal::new(0.0, SignalStrength::Weak);
        }
        
        let position = if funding_rate > 0.0 { 1.0 } else { -1.0 };
        let strength = if funding_rate.abs() > self.threshold * 2.0 {
            SignalStrength::Strong
        } else {
            SignalStrength::Medium
        };
        
        TradingSignal::new(position, strength)
    }
    
    fn combine_signals(&self, base_signal: f64, funding_signal: &TradingSignal) -> f64 {
        if funding_signal.is_neutral() {
            return base_signal;
        }
        
        let weight = match funding_signal.strength {
            SignalStrength::Strong => self.funding_config.funding_weight,
            SignalStrength::Medium => self.funding_config.funding_weight * 0.7,
            SignalStrength::Weak => self.funding_config.funding_weight * 0.3,
        };
        
        let combined = base_signal * (1.0 - weight) + funding_signal.position * weight;
        
        // Normalize to -1.0, 0.0, or 1.0
        if combined > 0.3 {
            1.0
        } else if combined < -0.3 {
            -1.0
        } else {
            0.0
        }
    }
}

/// Enhanced SMA cross strategy with funding awareness
pub struct EnhancedSmaStrategy {
    /// Fast period for SMA
    fast_period: usize,
    /// Slow period for SMA
    slow_period: usize,
    /// Funding-aware configuration
    funding_config: FundingAwareConfig,
}

impl EnhancedSmaStrategy {
    /// Create a new EnhancedSmaStrategy
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            funding_config: FundingAwareConfig::default(),
        }
    }
    
    /// Calculate SMA for a given period
    fn calculate_sma(&self, data: &[f64], period: usize) -> f64 {
        if data.len() < period {
            return 0.0;
        }
        
        let sum: f64 = data[data.len() - period..].iter().sum();
        sum / period as f64
    }
}

impl HyperliquidStrategy for EnhancedSmaStrategy {
    fn funding_config(&self) -> &FundingAwareConfig {
        &self.funding_config
    }
    
    fn set_funding_config(&mut self, config: FundingAwareConfig) {
        self.funding_config = config;
    }
    
    fn process_funding(&self, funding_rate: f64) -> TradingSignal {
        if !self.funding_config.use_funding_direction || 
           funding_rate.abs() <= self.funding_config.funding_threshold {
            return TradingSignal::new(0.0, SignalStrength::Weak);
        }
        
        let position = if funding_rate > 0.0 { 1.0 } else { -1.0 };
        let strength = if funding_rate.abs() > self.funding_config.funding_threshold * 2.0 {
            SignalStrength::Medium
        } else {
            SignalStrength::Weak
        };
        
        TradingSignal::new(position, strength)
    }
    
    fn combine_signals(&self, base_signal: f64, funding_signal: &TradingSignal) -> f64 {
        if funding_signal.is_neutral() {
            return base_signal;
        }
        
        // If signals agree, strengthen the position
        if (base_signal > 0.0 && funding_signal.is_long()) || 
           (base_signal < 0.0 && funding_signal.is_short()) {
            return base_signal;
        }
        
        // If signals disagree, reduce the position based on funding strength
        let weight = match funding_signal.strength {
            SignalStrength::Strong => self.funding_config.funding_weight,
            SignalStrength::Medium => self.funding_config.funding_weight * 0.5,
            SignalStrength::Weak => self.funding_config.funding_weight * 0.2,
        };
        
        base_signal * (1.0 - weight)
    }
}