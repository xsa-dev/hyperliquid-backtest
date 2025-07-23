use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, FixedOffset};
use tracing::{info, warn};

use crate::backtest::HyperliquidBacktest;
use crate::data::HyperliquidData;
use crate::strategies::HyperliquidStrategy;
use crate::trading_mode::{
    TradingMode, TradingModeError, TradingConfig
};
use crate::unified_data::{
    Position, OrderResult
};

/// Result of strategy execution
#[derive(Debug)]
pub struct TradingResult {
    /// Trading mode used for execution
    pub mode: TradingMode,
    
    /// Final portfolio value
    pub portfolio_value: f64,
    
    /// Profit and loss
    pub pnl: f64,
    
    /// Trading PnL (excluding funding)
    pub trading_pnl: Option<f64>,
    
    /// Funding PnL
    pub funding_pnl: Option<f64>,
    
    /// Number of trades executed
    pub trade_count: usize,
    
    /// Win rate (percentage)
    pub win_rate: f64,
    
    /// Execution timestamp
    pub timestamp: DateTime<FixedOffset>,
    
    /// Mode-specific result data
    pub mode_specific_data: HashMap<String, String>,
}



/// Trading mode manager for seamless transitions between trading modes
pub struct TradingModeManager {
    /// Current trading mode
    current_mode: TradingMode,
    
    /// Trading configuration
    config: TradingConfig,
    
    /// Backtest data (for backtest mode)
    backtest_data: Option<Arc<HyperliquidData>>,
    
    /// Backtest instance (for backtest mode)
    backtest_instance: Option<Arc<Mutex<HyperliquidBacktest>>>,
    
    /// Paper trading engine (for paper trading mode)
    paper_trading_engine: Option<Arc<Mutex<()>>>, // Placeholder for future implementation
    
    /// Live trading engine (for live trading mode)
    live_trading_engine: Option<Arc<Mutex<()>>>, // Placeholder for future implementation
    
    /// Current positions across all modes
    positions: HashMap<String, Position>,
    
    /// Trading history
    trading_history: Vec<OrderResult>,
}

impl TradingModeManager {
    /// Create a new trading mode manager with the specified mode and configuration
    pub fn new(mode: TradingMode, config: TradingConfig) -> Self {
        // Validate configuration for the initial mode
        if let Err(e) = config.validate_for_mode(mode) {
            warn!("Invalid configuration for {}: {}", mode, e);
        }
        
        info!("Initializing trading mode manager in {} mode", mode);
        
        Self {
            current_mode: mode,
            config,
            backtest_data: None,
            backtest_instance: None,
            paper_trading_engine: None,
            live_trading_engine: None,
            positions: HashMap::new(),
            trading_history: Vec::new(),
        }
    }
    
    /// Get the current trading mode
    pub fn current_mode(&self) -> TradingMode {
        self.current_mode
    }
    
    /// Get the current trading configuration
    pub fn config(&self) -> &TradingConfig {
        &self.config
    }
    
    /// Get a mutable reference to the trading configuration
    pub fn config_mut(&mut self) -> &mut TradingConfig {
        &mut self.config
    }
    
    /// Update the trading configuration
    pub fn update_config(&mut self, config: TradingConfig) -> std::result::Result<(), TradingModeError> {
        // Validate the new configuration for the current mode
        config.validate_for_mode(self.current_mode)?;
        
        info!("Updating trading configuration");
        self.config = config;
        
        Ok(())
    }
    
    /// Switch to a different trading mode
    pub fn switch_mode(&mut self, mode: TradingMode) -> std::result::Result<(), TradingModeError> {
        if self.current_mode == mode {
            info!("Already in {} mode, no switch needed", mode);
            return Ok(());
        }
        
        // Validate configuration for the new mode
        self.config.validate_for_mode(mode)?;
        
        // Check if the mode transition is supported
        match (self.current_mode, mode) {
            // Allow transitions from backtest to paper trade
            (TradingMode::Backtest, TradingMode::PaperTrade) => {
                info!("Switching from {} mode to {} mode", self.current_mode, mode);
                // Additional logic for transitioning from backtest to paper trade
            },
            
            // Allow transitions from paper trade to live trade
            (TradingMode::PaperTrade, TradingMode::LiveTrade) => {
                info!("Switching from {} mode to {} mode", self.current_mode, mode);
                // Additional safety checks for live trading
                if self.config.risk_config.is_none() {
                    return Err(TradingModeError::InvalidConfiguration {
                        mode,
                        message: "Risk configuration is required for live trading".to_string(),
                    });
                }
                
                if self.config.api_config.is_none() {
                    return Err(TradingModeError::InvalidConfiguration {
                        mode,
                        message: "API configuration is required for live trading".to_string(),
                    });
                }
                
                // Additional logic for transitioning from paper trade to live trade
            },
            
            // Allow transitions from live trade to paper trade (safety feature)
            (TradingMode::LiveTrade, TradingMode::PaperTrade) => {
                info!("Switching from {} mode to {} mode", self.current_mode, mode);
                // Additional logic for transitioning from live trade to paper trade
            },
            
            // Allow transitions to backtest mode from any mode
            (_, TradingMode::Backtest) => {
                info!("Switching from {} mode to {} mode", self.current_mode, mode);
                // Additional logic for transitioning to backtest mode
            },
            
            // Disallow direct transition from backtest to live trade
            (TradingMode::Backtest, TradingMode::LiveTrade) => {
                return Err(TradingModeError::UnsupportedModeTransition {
                    from: self.current_mode,
                    to: mode,
                });
            },
            
            // Handle any other transitions
            _ => {
                info!("Switching from {} mode to {} mode", self.current_mode, mode);
            },
        }
        
        // Update the current mode
        self.current_mode = mode;
        
        // Initialize mode-specific components if needed
        match mode {
            TradingMode::Backtest => {
                // Initialize backtest components if needed
            },
            TradingMode::PaperTrade => {
                // Initialize paper trading components if needed
                if self.paper_trading_engine.is_none() {
                    // Placeholder for future implementation
                    self.paper_trading_engine = Some(Arc::new(Mutex::new(())));
                }
            },
            TradingMode::LiveTrade => {
                // Initialize live trading components if needed
                if self.live_trading_engine.is_none() {
                    // Placeholder for future implementation
                    self.live_trading_engine = Some(Arc::new(Mutex::new(())));
                }
            },
        }
        
        Ok(())
    }
    
    /// Set historical data for backtesting
    pub fn set_backtest_data(&mut self, data: HyperliquidData) -> std::result::Result<(), TradingModeError> {
        info!("Setting backtest data for {}", data.symbol);
        self.backtest_data = Some(Arc::new(data));
        Ok(())
    }
    
    /// Execute a strategy in the current trading mode
    pub fn execute_strategy<S>(&mut self, strategy: S) -> std::result::Result<TradingResult, TradingModeError>
    where
        S: HyperliquidStrategy + 'static,
    {
        match self.current_mode {
            TradingMode::Backtest => {
                self.execute_backtest_strategy(strategy)
            },
            TradingMode::PaperTrade => {
                Err(TradingModeError::NotImplemented("Paper trading execution".to_string()))
            },
            TradingMode::LiveTrade => {
                Err(TradingModeError::NotImplemented("Live trading execution".to_string()))
            },
        }
    }
    
    /// Execute a strategy in backtest mode
    fn execute_backtest_strategy<S>(&mut self, strategy: S) -> std::result::Result<TradingResult, TradingModeError>
    where
        S: HyperliquidStrategy + 'static,
    {
        // Check if we have backtest data
        let data = match &self.backtest_data {
            Some(data) => data.clone(),
            None => {
                return Err(TradingModeError::StrategyExecutionError {
                    mode: TradingMode::Backtest,
                    message: "No backtest data available".to_string(),
                });
            }
        };
        
        // Create a new backtest instance
        let mut backtest = HyperliquidBacktest::new(
            (*data).clone(),
            "Custom Strategy".to_string(),
            self.config.initial_balance,
            crate::backtest::HyperliquidCommission::default(),
        );
        
        // Run the backtest with funding calculations
        backtest.calculate_with_funding()?;
        
        // Get the backtest report
        let report = backtest.enhanced_report()?;
        
        // Store the backtest instance for later reference
        self.backtest_instance = Some(Arc::new(Mutex::new(backtest)));
        
        // Create a trading result from the backtest report
        let result = TradingResult {
            mode: TradingMode::Backtest,
            portfolio_value: report.final_equity,
            pnl: report.total_return * self.config.initial_balance, // Calculate PnL from return
            trading_pnl: None, // Will be calculated from report if available
            funding_pnl: None, // Will be calculated from report if available
            trade_count: report.trade_count,
            win_rate: report.win_rate,
            timestamp: chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()),
            mode_specific_data: {
                let mut data = HashMap::new();
                data.insert("sharpe_ratio".to_string(), report.sharpe_ratio.to_string());
                data.insert("max_drawdown".to_string(), report.max_drawdown.to_string());
                data.insert("total_return".to_string(), report.total_return.to_string());
                data
            },
        };
        
        Ok(result)
    }
    
    /// Get current positions
    pub fn get_positions(&self) -> &HashMap<String, Position> {
        &self.positions
    }
    
    /// Get trading history
    pub fn get_trading_history(&self) -> &[OrderResult] {
        &self.trading_history
    }
}