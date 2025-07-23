//! # Trading Mode Management
//!
//! This module provides functionality for managing different trading modes (backtest, paper trading, live trading)
//! and seamlessly transitioning between them while maintaining consistent strategy execution.
//!
//! ## Features
//!
//! - Unified trading mode interface across backtest, paper trading, and live trading
//! - Configuration management for different trading modes
//! - Seamless strategy execution across all modes
//! - Mode-specific configuration and safety checks

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{warn, error};

use crate::errors::HyperliquidBacktestError;

/// Represents the different trading modes available in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradingMode {
    /// Backtesting mode using historical data
    Backtest,
    
    /// Paper trading mode using real-time data but simulated execution
    PaperTrade,
    
    /// Live trading mode using real-time data and real order execution
    LiveTrade,
}

impl std::fmt::Display for TradingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradingMode::Backtest => write!(f, "Backtest"),
            TradingMode::PaperTrade => write!(f, "Paper Trading"),
            TradingMode::LiveTrade => write!(f, "Live Trading"),
        }
    }
}

/// Error types specific to trading mode operations
#[derive(Debug, Error)]
pub enum TradingModeError {
    /// Error when switching to an unsupported mode
    #[error("Unsupported trading mode transition from {from} to {to}")]
    UnsupportedModeTransition {
        from: TradingMode,
        to: TradingMode,
    },
    
    /// Error when configuration is missing for a specific mode
    #[error("Missing configuration for {0} mode")]
    MissingConfiguration(TradingMode),
    
    /// Error when executing a strategy
    #[error("Strategy execution error in {mode} mode: {message}")]
    StrategyExecutionError {
        mode: TradingMode,
        message: String,
    },
    
    /// Error when validating configuration
    #[error("Invalid configuration for {mode} mode: {message}")]
    InvalidConfiguration {
        mode: TradingMode,
        message: String,
    },
    
    /// Wrapper for backtesting errors
    #[error("Backtesting error: {0}")]
    BacktestError(#[from] HyperliquidBacktestError),
    
    /// Error when a feature is not yet implemented
    #[error("Feature not implemented: {0}")]
    NotImplemented(String),
}

/// Configuration for trading modes
#[derive(Debug, Clone)]
pub struct TradingConfig {
    /// Initial balance for trading
    pub initial_balance: f64,
    
    /// Risk management configuration
    pub risk_config: Option<RiskConfig>,
    
    /// Slippage configuration for paper trading
    pub slippage_config: Option<SlippageConfig>,
    
    /// API configuration for live trading
    pub api_config: Option<ApiConfig>,
    
    /// Additional mode-specific configuration parameters
    pub parameters: HashMap<String, String>,
}

impl TradingConfig {
    /// Create a new trading configuration with the specified initial balance
    pub fn new(initial_balance: f64) -> Self {
        Self {
            initial_balance,
            risk_config: None,
            slippage_config: None,
            api_config: None,
            parameters: HashMap::new(),
        }
    }
    
    /// Add a risk configuration
    pub fn with_risk_config(mut self, risk_config: RiskConfig) -> Self {
        self.risk_config = Some(risk_config);
        self
    }
    
    /// Add a slippage configuration for paper trading
    pub fn with_slippage_config(mut self, slippage_config: SlippageConfig) -> Self {
        self.slippage_config = Some(slippage_config);
        self
    }
    
    /// Add an API configuration for live trading
    pub fn with_api_config(mut self, api_config: ApiConfig) -> Self {
        self.api_config = Some(api_config);
        self
    }
    
    /// Add a custom parameter
    pub fn with_parameter(mut self, key: &str, value: &str) -> Self {
        self.parameters.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Validate the configuration for a specific trading mode
    pub fn validate_for_mode(&self, mode: TradingMode) -> std::result::Result<(), TradingModeError> {
        match mode {
            TradingMode::Backtest => {
                // Backtesting mode has minimal requirements
                if self.initial_balance <= 0.0 {
                    return Err(TradingModeError::InvalidConfiguration {
                        mode,
                        message: "Initial balance must be positive".to_string(),
                    });
                }
            },
            TradingMode::PaperTrade => {
                // Paper trading requires initial balance and should have slippage config
                if self.initial_balance <= 0.0 {
                    return Err(TradingModeError::InvalidConfiguration {
                        mode,
                        message: "Initial balance must be positive".to_string(),
                    });
                }
                
                if self.slippage_config.is_none() {
                    warn!("No slippage configuration provided for paper trading mode. Using default values.");
                }
            },
            TradingMode::LiveTrade => {
                // Live trading requires initial balance, risk config, and API config
                if self.initial_balance <= 0.0 {
                    return Err(TradingModeError::InvalidConfiguration {
                        mode,
                        message: "Initial balance must be positive".to_string(),
                    });
                }
                
                if self.risk_config.is_none() {
                    return Err(TradingModeError::InvalidConfiguration {
                        mode,
                        message: "Risk configuration is required for live trading".to_string(),
                    });
                }
                
                if self.api_config.is_none() {
                    return Err(TradingModeError::InvalidConfiguration {
                        mode,
                        message: "API configuration is required for live trading".to_string(),
                    });
                }
            },
        }
        
        Ok(())
    }
}

/// Risk management configuration
#[derive(Debug, Clone)]
pub struct RiskConfig {
    /// Maximum position size as a percentage of portfolio value
    pub max_position_size_pct: f64,
    
    /// Maximum daily loss as a percentage of portfolio value
    pub max_daily_loss_pct: f64,
    
    /// Stop loss percentage for positions
    pub stop_loss_pct: f64,
    
    /// Take profit percentage for positions
    pub take_profit_pct: f64,
    
    /// Maximum leverage allowed
    pub max_leverage: f64,
    
    /// Maximum portfolio concentration in a single asset class (percentage)
    pub max_concentration_pct: f64,
    
    /// Maximum correlation between positions (0.0 to 1.0)
    pub max_correlation_pct: f64,
    
    /// Maximum portfolio volatility (percentage)
    pub max_portfolio_volatility_pct: f64,
    
    /// Volatility-based position sizing factor (0.0 to 1.0)
    pub volatility_sizing_factor: f64,
    
    /// Maximum drawdown before emergency stop (percentage)
    pub max_drawdown_pct: f64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_size_pct: 0.1,  // 10% of portfolio
            max_daily_loss_pct: 0.02,    // 2% max daily loss
            stop_loss_pct: 0.05,         // 5% stop loss
            take_profit_pct: 0.1,        // 10% take profit
            max_leverage: 3.0,           // 3x max leverage
            max_concentration_pct: 0.25, // 25% max concentration in one asset class
            max_correlation_pct: 0.7, // 0.7 maximum correlation between positions
            max_portfolio_volatility_pct: 0.2, // 20% maximum portfolio volatility
            volatility_sizing_factor: 0.5, // 50% volatility-based position sizing
            max_drawdown_pct: 0.15,     // 15% maximum drawdown before emergency stop
        }
    }
}

/// Slippage simulation configuration for paper trading
#[derive(Debug, Clone)]
pub struct SlippageConfig {
    /// Base slippage as a percentage
    pub base_slippage_pct: f64,
    
    /// Volume-based slippage factor
    pub volume_impact_factor: f64,
    
    /// Volatility-based slippage factor
    pub volatility_impact_factor: f64,
    
    /// Random slippage component maximum (percentage)
    pub random_slippage_max_pct: f64,
    
    /// Simulated latency in milliseconds
    pub simulated_latency_ms: u64,
}

impl Default for SlippageConfig {
    fn default() -> Self {
        Self {
            base_slippage_pct: 0.0005,   // 0.05% base slippage
            volume_impact_factor: 0.1,   // Volume impact factor
            volatility_impact_factor: 0.2, // Volatility impact factor
            random_slippage_max_pct: 0.001, // 0.1% max random component
            simulated_latency_ms: 500,   // 500ms simulated latency
        }
    }
}

/// API configuration for live trading
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// API key for authentication
    pub api_key: String,
    
    /// API secret for authentication
    pub api_secret: String,
    
    /// API endpoint URL
    pub endpoint: String,
    
    /// Whether to use testnet
    pub use_testnet: bool,
    
    /// Timeout for API requests in milliseconds
    pub timeout_ms: u64,
}

// Additional types and implementations will be in trading_mode_impl.rs