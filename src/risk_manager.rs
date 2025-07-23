//! # Risk Management System
//!
//! This module provides risk management functionality for trading strategies,
//! including position size limits, maximum daily loss protection, stop-loss and
//! take-profit mechanisms, and leverage limits.

use std::collections::HashMap;
use chrono::{DateTime, FixedOffset, Utc};
use thiserror::Error;
use tracing::{info, warn, error};

use crate::trading_mode::{RiskConfig};
use crate::unified_data::{Position, OrderRequest, OrderSide, OrderType};

/// Error types specific to risk management operations
#[derive(Debug, Error)]
pub enum RiskError {
    /// Error when position size exceeds limits
    #[error("Position size exceeds limit: {message}")]
    PositionSizeExceeded {
        message: String,
    },
    
    /// Error when daily loss limit is reached
    #[error("Daily loss limit reached: {current_loss_pct}% exceeds {max_loss_pct}%")]
    DailyLossLimitReached {
        current_loss_pct: f64,
        max_loss_pct: f64,
    },
    
    /// Error when leverage limit is exceeded
    #[error("Leverage limit exceeded: {current_leverage}x exceeds {max_leverage}x")]
    LeverageLimitExceeded {
        current_leverage: f64,
        max_leverage: f64,
    },
    
    /// Error when margin is insufficient
    #[error("Insufficient margin: {required_margin} exceeds {available_margin}")]
    InsufficientMargin {
        required_margin: f64,
        available_margin: f64,
    },
    
    /// Error when portfolio concentration limit is exceeded
    #[error("Portfolio concentration limit exceeded: {asset_class} at {concentration_pct}% exceeds {max_concentration_pct}%")]
    ConcentrationLimitExceeded {
        asset_class: String,
        concentration_pct: f64,
        max_concentration_pct: f64,
    },
    
    /// Error when position correlation limit is exceeded
    #[error("Position correlation limit exceeded: {symbol1} and {symbol2} correlation {correlation} exceeds {max_correlation}")]
    CorrelationLimitExceeded {
        symbol1: String,
        symbol2: String,
        correlation: f64,
        max_correlation: f64,
    },
    
    /// Error when portfolio volatility limit is exceeded
    #[error("Portfolio volatility limit exceeded: {current_volatility_pct}% exceeds {max_volatility_pct}%")]
    VolatilityLimitExceeded {
        current_volatility_pct: f64,
        max_volatility_pct: f64,
    },
    
    /// Error when drawdown limit is exceeded
    #[error("Drawdown limit exceeded: {current_drawdown_pct}% exceeds {max_drawdown_pct}%")]
    DrawdownLimitExceeded {
        current_drawdown_pct: f64,
        max_drawdown_pct: f64,
    },
    
    /// General risk management error
    #[error("Risk management error: {0}")]
    General(String),
}

/// Result type for risk management operations
pub type Result<T> = std::result::Result<T, RiskError>;

/// Stop-loss or take-profit order
#[derive(Debug, Clone)]
pub struct RiskOrder {
    /// Original order ID this risk order is associated with
    pub parent_order_id: String,
    
    /// Symbol/ticker of the asset
    pub symbol: String,
    
    /// Order side (buy/sell)
    pub side: OrderSide,
    
    /// Order type
    pub order_type: OrderType,
    
    /// Order quantity
    pub quantity: f64,
    
    /// Trigger price
    pub trigger_price: f64,
    
    /// Whether this is a stop-loss order
    pub is_stop_loss: bool,
    
    /// Whether this is a take-profit order
    pub is_take_profit: bool,
}

/// Daily risk tracking
#[derive(Debug, Clone)]
struct DailyRiskTracker {
    /// Date of tracking
    date: chrono::NaiveDate,
    
    /// Starting portfolio value
    starting_value: f64,
    
    /// Current portfolio value
    current_value: f64,
    
    /// Realized profit/loss for the day
    realized_pnl: f64,
    
    /// Unrealized profit/loss for the day
    unrealized_pnl: f64,
    
    /// Maximum drawdown for the day
    max_drawdown: f64,
    
    /// Highest portfolio value for the day
    highest_value: f64,
}

impl DailyRiskTracker {
    /// Create a new daily risk tracker
    fn new(portfolio_value: f64) -> Self {
        Self {
            date: Utc::now().date_naive(),
            starting_value: portfolio_value,
            current_value: portfolio_value,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
            max_drawdown: 0.0,
            highest_value: portfolio_value,
        }
    }
    
    /// Update the tracker with new portfolio value
    fn update(&mut self, portfolio_value: f64, realized_pnl_delta: f64) {
        self.current_value = portfolio_value;
        self.realized_pnl += realized_pnl_delta;
        self.unrealized_pnl = portfolio_value - self.starting_value - self.realized_pnl;
        
        // Update highest value if needed
        if portfolio_value > self.highest_value {
            self.highest_value = portfolio_value;
        }
        
        // Update max drawdown if needed
        let current_drawdown = (self.highest_value - portfolio_value) / self.highest_value;
        if current_drawdown > self.max_drawdown {
            self.max_drawdown = current_drawdown;
        }
    }
    
    /// Check if the daily loss limit is reached
    fn is_daily_loss_limit_reached(&self, max_daily_loss_pct: f64) -> bool {
        let daily_loss_pct = (self.starting_value - self.current_value) / self.starting_value * 100.0;
        daily_loss_pct >= max_daily_loss_pct
    }
    
    /// Get the current daily loss percentage
    fn daily_loss_pct(&self) -> f64 {
        (self.starting_value - self.current_value) / self.starting_value * 100.0
    }
    
    /// Reset the tracker for a new day
    fn reset(&mut self, portfolio_value: f64) {
        self.date = Utc::now().date_naive();
        self.starting_value = portfolio_value;
        self.current_value = portfolio_value;
        self.realized_pnl = 0.0;
        self.unrealized_pnl = 0.0;
        self.max_drawdown = 0.0;
        self.highest_value = portfolio_value;
    }
}

/// Asset class for correlation and concentration management
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssetClass {
    /// Cryptocurrencies
    Crypto,
    
    /// Stablecoins
    Stablecoin,
    
    /// Defi tokens
    Defi,
    
    /// Layer 1 blockchain tokens
    Layer1,
    
    /// Layer 2 scaling solution tokens
    Layer2,
    
    /// Meme coins
    Meme,
    
    /// NFT related tokens
    NFT,
    
    /// Gaming tokens
    Gaming,
    
    /// Other tokens
    Other,
}

/// Historical volatility data for an asset
#[derive(Debug, Clone)]
pub struct VolatilityData {
    /// Symbol of the asset
    pub symbol: String,
    
    /// Daily volatility (percentage)
    pub daily_volatility: f64,
    
    /// Weekly volatility (percentage)
    pub weekly_volatility: f64,
    
    /// Monthly volatility (percentage)
    pub monthly_volatility: f64,
    
    /// Historical price data for volatility calculation
    pub price_history: Vec<f64>,
    
    /// Last update timestamp
    pub last_update: DateTime<FixedOffset>,
}

/// Correlation data between two assets
#[derive(Debug, Clone)]
pub struct CorrelationData {
    /// First symbol
    pub symbol1: String,
    
    /// Second symbol
    pub symbol2: String,
    
    /// Correlation coefficient (-1.0 to 1.0)
    pub correlation: f64,
    
    /// Last update timestamp
    pub last_update: DateTime<FixedOffset>,
}

/// Portfolio metrics for risk management
#[derive(Debug, Clone)]
pub struct PortfolioMetrics {
    /// Portfolio value
    pub value: f64,
    
    /// Portfolio volatility (percentage)
    pub volatility: f64,
    
    /// Maximum drawdown (percentage)
    pub max_drawdown: f64,
    
    /// Value at Risk (VaR) at 95% confidence
    pub var_95: f64,
    
    /// Value at Risk (VaR) at 99% confidence
    pub var_99: f64,
    
    /// Concentration by asset class
    pub concentration: HashMap<AssetClass, f64>,
}

/// Risk manager for trading strategies
#[derive(Debug)]
pub struct RiskManager {
    /// Risk configuration
    config: RiskConfig,
    
    /// Current portfolio value
    portfolio_value: f64,
    
    /// Available margin
    available_margin: f64,
    
    /// Daily risk tracker
    daily_tracker: DailyRiskTracker,
    
    /// Stop-loss orders
    stop_loss_orders: HashMap<String, RiskOrder>,
    
    /// Take-profit orders
    take_profit_orders: HashMap<String, RiskOrder>,
    
    /// Emergency stop flag
    emergency_stop: bool,
    
    /// Asset class mapping
    asset_classes: HashMap<String, AssetClass>,
    
    /// Volatility data by symbol
    volatility_data: HashMap<String, VolatilityData>,
    
    /// Correlation data between symbols
    correlation_data: HashMap<(String, String), CorrelationData>,
    
    /// Portfolio metrics
    portfolio_metrics: PortfolioMetrics,
    
    /// Historical portfolio values for drawdown calculation
    historical_portfolio_values: Vec<(DateTime<FixedOffset>, f64)>,
}

impl RiskManager {
    /// Create a new risk manager with the specified configuration
    pub fn new(config: RiskConfig, initial_portfolio_value: f64) -> Self {
        let available_margin = initial_portfolio_value;
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        
        Self {
            config,
            portfolio_value: initial_portfolio_value,
            available_margin,
            daily_tracker: DailyRiskTracker::new(initial_portfolio_value),
            stop_loss_orders: HashMap::new(),
            take_profit_orders: HashMap::new(),
            emergency_stop: false,
            asset_classes: Self::default_asset_classes(),
            volatility_data: HashMap::new(),
            correlation_data: HashMap::new(),
            portfolio_metrics: PortfolioMetrics {
                value: initial_portfolio_value,
                volatility: 0.0,
                max_drawdown: 0.0,
                var_95: 0.0,
                var_99: 0.0,
                concentration: HashMap::new(),
            },
            historical_portfolio_values: vec![(now, initial_portfolio_value)],
        }
    }
    
    /// Create a new risk manager with default configuration
    pub fn default(initial_portfolio_value: f64) -> Self {
        Self::new(RiskConfig::default(), initial_portfolio_value)
    }
    
    /// Create default asset class mappings
    fn default_asset_classes() -> HashMap<String, AssetClass> {
        let mut map = HashMap::new();
        
        // Major cryptocurrencies
        map.insert("BTC".to_string(), AssetClass::Crypto);
        map.insert("ETH".to_string(), AssetClass::Crypto);
        map.insert("BNB".to_string(), AssetClass::Crypto);
        map.insert("SOL".to_string(), AssetClass::Crypto);
        map.insert("XRP".to_string(), AssetClass::Crypto);
        map.insert("ADA".to_string(), AssetClass::Crypto);
        map.insert("AVAX".to_string(), AssetClass::Crypto);
        
        // Stablecoins
        map.insert("USDT".to_string(), AssetClass::Stablecoin);
        map.insert("USDC".to_string(), AssetClass::Stablecoin);
        map.insert("DAI".to_string(), AssetClass::Stablecoin);
        map.insert("BUSD".to_string(), AssetClass::Stablecoin);
        
        // DeFi tokens
        map.insert("UNI".to_string(), AssetClass::Defi);
        map.insert("AAVE".to_string(), AssetClass::Defi);
        map.insert("MKR".to_string(), AssetClass::Defi);
        map.insert("COMP".to_string(), AssetClass::Defi);
        map.insert("SNX".to_string(), AssetClass::Defi);
        map.insert("SUSHI".to_string(), AssetClass::Defi);
        
        // Layer 1 blockchains
        map.insert("DOT".to_string(), AssetClass::Layer1);
        map.insert("ATOM".to_string(), AssetClass::Layer1);
        map.insert("NEAR".to_string(), AssetClass::Layer1);
        map.insert("ALGO".to_string(), AssetClass::Layer1);
        
        // Layer 2 solutions
        map.insert("MATIC".to_string(), AssetClass::Layer2);
        map.insert("LRC".to_string(), AssetClass::Layer2);
        map.insert("OMG".to_string(), AssetClass::Layer2);
        map.insert("IMX".to_string(), AssetClass::Layer2);
        
        // Meme coins
        map.insert("DOGE".to_string(), AssetClass::Meme);
        map.insert("SHIB".to_string(), AssetClass::Meme);
        map.insert("PEPE".to_string(), AssetClass::Meme);
        
        // NFT related
        map.insert("APE".to_string(), AssetClass::NFT);
        map.insert("SAND".to_string(), AssetClass::NFT);
        map.insert("MANA".to_string(), AssetClass::NFT);
        
        // Gaming
        map.insert("AXS".to_string(), AssetClass::Gaming);
        map.insert("ENJ".to_string(), AssetClass::Gaming);
        map.insert("GALA".to_string(), AssetClass::Gaming);
        
        map
    }
    
    /// Get the current risk configuration
    pub fn config(&self) -> &RiskConfig {
        &self.config
    }
    
    /// Update the risk configuration
    pub fn update_config(&mut self, config: RiskConfig) {
        info!("Updating risk configuration");
        self.config = config;
    }
    
    /// Update portfolio value and check daily risk limits
    pub fn update_portfolio_value(&mut self, new_value: f64, realized_pnl_delta: f64) -> Result<()> {
        // Check if we need to reset the daily tracker (new day)
        let current_date = Utc::now().date_naive();
        if current_date != self.daily_tracker.date {
            info!("New trading day, resetting daily risk tracker");
            self.daily_tracker.reset(new_value);
        } else {
            // Update the daily tracker
            self.daily_tracker.update(new_value, realized_pnl_delta);
            
            // Check if daily loss limit is reached
            if self.daily_tracker.is_daily_loss_limit_reached(self.config.max_daily_loss_pct) {
                let daily_loss_pct = self.daily_tracker.daily_loss_pct();
                warn!("Daily loss limit reached: {:.2}% exceeds {:.2}%", 
                      daily_loss_pct, self.config.max_daily_loss_pct);
                
                // Set emergency stop
                self.emergency_stop = true;
                
                return Err(RiskError::DailyLossLimitReached {
                    current_loss_pct: daily_loss_pct,
                    max_loss_pct: self.config.max_daily_loss_pct,
                });
            }
        }
        
        // Update portfolio value and available margin
        self.portfolio_value = new_value;
        self.available_margin = new_value; // Simplified, in reality would depend on existing positions
        
        Ok(())
    }
    
    /// Validate an order against risk limits
    pub fn validate_order(&self, order: &OrderRequest, current_positions: &HashMap<String, Position>) -> Result<()> {
        // Check if emergency stop is active
        if self.emergency_stop {
            return Err(RiskError::General("Emergency stop is active".to_string()));
        }
        
        // Check daily loss limit
        if self.daily_tracker.is_daily_loss_limit_reached(self.config.max_daily_loss_pct) {
            let daily_loss_pct = self.daily_tracker.daily_loss_pct();
            return Err(RiskError::DailyLossLimitReached {
                current_loss_pct: daily_loss_pct,
                max_loss_pct: self.config.max_daily_loss_pct,
            });
        }
        
        // Check drawdown limit
        if self.portfolio_metrics.max_drawdown > self.config.max_drawdown_pct {
            return Err(RiskError::DrawdownLimitExceeded {
                current_drawdown_pct: self.portfolio_metrics.max_drawdown * 100.0,
                max_drawdown_pct: self.config.max_drawdown_pct * 100.0,
            });
        }
        
        // Calculate order value
        let order_value = match order.price {
            Some(price) => order.quantity * price,
            None => {
                // For market orders, we need to estimate the price
                // In a real implementation, we would use the current market price
                // For simplicity, we'll use the current position price if available
                if let Some(position) = current_positions.get(&order.symbol) {
                    order.quantity * position.current_price
                } else {
                    // If no position exists, we can't validate the order properly
                    return Err(RiskError::General(
                        "Cannot validate market order without price information".to_string()
                    ));
                }
            }
        };
        
        // Check position size limit
        let max_position_value = self.portfolio_value * self.config.max_position_size_pct;
        
        // Calculate the total position value after this order
        let mut new_position_value = order_value;
        if let Some(position) = current_positions.get(&order.symbol) {
            // Add existing position value
            new_position_value = match order.side {
                OrderSide::Buy => position.size.abs() * position.current_price + order_value,
                OrderSide::Sell => {
                    if order.quantity <= position.size {
                        // Reducing position
                        (position.size - order.quantity).abs() * position.current_price
                    } else {
                        // Flipping position
                        (order.quantity - position.size).abs() * position.current_price
                    }
                }
            };
        }
        
        // Apply volatility-based position sizing if volatility data is available
        if let Some(volatility_data) = self.volatility_data.get(&order.symbol) {
            let volatility_adjusted_max_size = self.calculate_volatility_adjusted_position_size(
                &order.symbol, 
                max_position_value
            );
            
            if new_position_value > volatility_adjusted_max_size {
                return Err(RiskError::PositionSizeExceeded {
                    message: format!(
                        "Position value ${:.2} exceeds volatility-adjusted limit ${:.2}",
                        new_position_value, volatility_adjusted_max_size
                    ),
                });
            }
        } else if new_position_value > max_position_value {
            return Err(RiskError::PositionSizeExceeded {
                message: format!(
                    "Position value ${:.2} exceeds limit ${:.2} ({:.2}% of portfolio)",
                    new_position_value, max_position_value, self.config.max_position_size_pct * 100.0
                ),
            });
        }
        
        // Check leverage limit
        let total_position_value = current_positions.values()
            .map(|p| p.size.abs() * p.current_price)
            .sum::<f64>() + order_value;
        
        let current_leverage = total_position_value / self.portfolio_value;
        if current_leverage > self.config.max_leverage {
            return Err(RiskError::LeverageLimitExceeded {
                current_leverage,
                max_leverage: self.config.max_leverage,
            });
        }
        
        // Check margin requirements
        let required_margin = total_position_value / self.config.max_leverage;
        if required_margin > self.available_margin {
            return Err(RiskError::InsufficientMargin {
                required_margin,
                available_margin: self.available_margin,
            });
        }
        
        // Check portfolio concentration limits
        if let Some(asset_class) = self.get_asset_class(&order.symbol) {
            let new_concentration = self.calculate_concentration_after_order(
                current_positions, 
                order, 
                asset_class
            );
            
            if new_concentration > self.config.max_concentration_pct {
                return Err(RiskError::ConcentrationLimitExceeded {
                    asset_class: format!("{:?}", asset_class),
                    concentration_pct: new_concentration * 100.0,
                    max_concentration_pct: self.config.max_concentration_pct * 100.0,
                });
            }
        }
        
        // Check correlation limits
        if let Err(e) = self.validate_correlation_limits(current_positions, order) {
            return Err(e);
        }
        
        // Check portfolio volatility limits
        if let Err(e) = self.validate_portfolio_volatility(current_positions, order) {
            return Err(e);
        }
        
        Ok(())
    }
    
    /// Generate stop-loss order for a position
    pub fn generate_stop_loss(&self, position: &Position, parent_order_id: &str) -> Option<RiskOrder> {
        if position.size == 0.0 {
            return None;
        }
        
        // Calculate stop loss price
        let stop_loss_price = if position.size > 0.0 {
            // Long position
            position.entry_price * (1.0 - self.config.stop_loss_pct)
        } else {
            // Short position
            position.entry_price * (1.0 + self.config.stop_loss_pct)
        };
        
        // Create stop loss order
        Some(RiskOrder {
            parent_order_id: parent_order_id.to_string(),
            symbol: position.symbol.clone(),
            side: if position.size > 0.0 { OrderSide::Sell } else { OrderSide::Buy },
            order_type: OrderType::StopMarket,
            quantity: position.size.abs(),
            trigger_price: stop_loss_price,
            is_stop_loss: true,
            is_take_profit: false,
        })
    }
    
    /// Generate take-profit order for a position
    pub fn generate_take_profit(&self, position: &Position, parent_order_id: &str) -> Option<RiskOrder> {
        if position.size == 0.0 {
            return None;
        }
        
        // Calculate take profit price
        let take_profit_price = if position.size > 0.0 {
            // Long position
            position.entry_price * (1.0 + self.config.take_profit_pct)
        } else {
            // Short position
            position.entry_price * (1.0 - self.config.take_profit_pct)
        };
        
        // Create take profit order
        Some(RiskOrder {
            parent_order_id: parent_order_id.to_string(),
            symbol: position.symbol.clone(),
            side: if position.size > 0.0 { OrderSide::Sell } else { OrderSide::Buy },
            order_type: OrderType::TakeProfitMarket,
            quantity: position.size.abs(),
            trigger_price: take_profit_price,
            is_stop_loss: false,
            is_take_profit: true,
        })
    }
    
    /// Register a stop-loss order
    pub fn register_stop_loss(&mut self, order: RiskOrder) {
        self.stop_loss_orders.insert(order.parent_order_id.clone(), order);
    }
    
    /// Register a take-profit order
    pub fn register_take_profit(&mut self, order: RiskOrder) {
        self.take_profit_orders.insert(order.parent_order_id.clone(), order);
    }
    
    /// Check if any stop-loss or take-profit orders should be triggered
    pub fn check_risk_orders(&mut self, current_prices: &HashMap<String, f64>) -> Vec<RiskOrder> {
        let mut triggered_orders = Vec::new();
        
        // Check stop-loss orders
        let mut triggered_stop_loss_ids = Vec::new();
        for (id, order) in &self.stop_loss_orders {
            if let Some(&current_price) = current_prices.get(&order.symbol) {
                let should_trigger = match order.side {
                    OrderSide::Sell => current_price <= order.trigger_price,
                    OrderSide::Buy => current_price >= order.trigger_price,
                };
                
                if should_trigger {
                    triggered_orders.push(order.clone());
                    triggered_stop_loss_ids.push(id.clone());
                }
            }
        }
        
        // Remove triggered stop-loss orders
        for id in triggered_stop_loss_ids {
            self.stop_loss_orders.remove(&id);
        }
        
        // Check take-profit orders
        let mut triggered_take_profit_ids = Vec::new();
        for (id, order) in &self.take_profit_orders {
            if let Some(&current_price) = current_prices.get(&order.symbol) {
                let should_trigger = match order.side {
                    OrderSide::Sell => current_price >= order.trigger_price,
                    OrderSide::Buy => current_price <= order.trigger_price,
                };
                
                if should_trigger {
                    triggered_orders.push(order.clone());
                    triggered_take_profit_ids.push(id.clone());
                }
            }
        }
        
        // Remove triggered take-profit orders
        for id in triggered_take_profit_ids {
            self.take_profit_orders.remove(&id);
        }
        
        triggered_orders
    }
    
    /// Check if trading should be stopped due to risk limits
    pub fn should_stop_trading(&self) -> bool {
        self.emergency_stop || 
        self.daily_tracker.is_daily_loss_limit_reached(self.config.max_daily_loss_pct)
    }
    
    /// Activate emergency stop
    pub fn activate_emergency_stop(&mut self) {
        warn!("Emergency stop activated");
        self.emergency_stop = true;
    }
    
    /// Deactivate emergency stop
    pub fn deactivate_emergency_stop(&mut self) {
        info!("Emergency stop deactivated");
        self.emergency_stop = false;
    }
    
    /// Get current daily risk metrics
    pub fn daily_risk_metrics(&self) -> (f64, f64, f64) {
        (
            self.daily_tracker.daily_loss_pct(),
            self.daily_tracker.max_drawdown * 100.0,
            (self.daily_tracker.realized_pnl / self.daily_tracker.starting_value) * 100.0
        )
    }
    
    /// Calculate required margin for a position
    pub fn calculate_required_margin(&self, position_value: f64) -> f64 {
        position_value / self.config.max_leverage
    }
    
    /// Get available margin
    pub fn available_margin(&self) -> f64 {
        self.available_margin
    }
    
    /// Update available margin
    pub fn update_available_margin(&mut self, margin: f64) {
        self.available_margin = margin;
    }
    
    /// Calculate volatility-adjusted position size
    pub fn calculate_volatility_adjusted_position_size(&self, symbol: &str, base_max_size: f64) -> f64 {
        if let Some(volatility_data) = self.volatility_data.get(symbol) {
            // Adjust position size based on volatility
            // Higher volatility = smaller position size
            let volatility_factor = 1.0 / (1.0 + volatility_data.daily_volatility / 100.0);
            base_max_size * volatility_factor
        } else {
            // If no volatility data, use base max size
            base_max_size
        }
    }
    
    /// Get asset class for a symbol
    pub fn get_asset_class(&self, symbol: &str) -> Option<&AssetClass> {
        self.asset_classes.get(symbol)
    }
    
    /// Calculate concentration after order
    pub fn calculate_concentration_after_order(
        &self,
        current_positions: &HashMap<String, Position>,
        order: &OrderRequest,
        asset_class: &AssetClass,
    ) -> f64 {
        let mut total_value = 0.0;
        let mut asset_class_value = 0.0;
        
        // Calculate current portfolio value by asset class
        for position in current_positions.values() {
            let position_value = position.size.abs() * position.current_price;
            total_value += position_value;
            
            if let Some(pos_asset_class) = self.asset_classes.get(&position.symbol) {
                if pos_asset_class == asset_class {
                    asset_class_value += position_value;
                }
            }
        }
        
        // Add the new order value if it's the same asset class
        if let Some(order_asset_class) = self.asset_classes.get(&order.symbol) {
            if order_asset_class == asset_class {
                let order_value = match order.price {
                    Some(price) => order.quantity * price,
                    None => {
                        // Estimate using current position price if available
                        if let Some(position) = current_positions.get(&order.symbol) {
                            order.quantity * position.current_price
                        } else {
                            0.0 // Can't calculate without price
                        }
                    }
                };
                asset_class_value += order_value;
            }
        }
        
        // Add the order value to total
        let order_value = match order.price {
            Some(price) => order.quantity * price,
            None => {
                if let Some(position) = current_positions.get(&order.symbol) {
                    order.quantity * position.current_price
                } else {
                    0.0
                }
            }
        };
        total_value += order_value;
        
        if total_value > 0.0 {
            asset_class_value / total_value
        } else {
            0.0
        }
    }
    
    /// Validate correlation limits
    pub fn validate_correlation_limits(
        &self,
        current_positions: &HashMap<String, Position>,
        order: &OrderRequest,
    ) -> Result<()> {
        // Check correlation with existing positions
        for position in current_positions.values() {
            if position.symbol != order.symbol {
                // Check if we have correlation data for this pair
                let key1 = (position.symbol.clone(), order.symbol.clone());
                let key2 = (order.symbol.clone(), position.symbol.clone());
                
                if let Some(correlation_data) = self.correlation_data.get(&key1)
                    .or_else(|| self.correlation_data.get(&key2)) {
                    
                    // If correlation is too high and positions are in same direction, reject
                    if correlation_data.correlation.abs() > self.config.max_correlation_pct {
                        let position_direction = if position.size > 0.0 { 1.0 } else { -1.0 };
                        let order_direction = match order.side {
                            OrderSide::Buy => 1.0,
                            OrderSide::Sell => -1.0,
                        };
                        
                        // If same direction and high correlation, it's risky
                        if position_direction * order_direction > 0.0 {
                            return Err(RiskError::CorrelationLimitExceeded {
                                symbol1: position.symbol.clone(),
                                symbol2: order.symbol.clone(),
                                correlation: correlation_data.correlation,
                                max_correlation: self.config.max_correlation_pct,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate portfolio volatility
    pub fn validate_portfolio_volatility(
        &self,
        current_positions: &HashMap<String, Position>,
        order: &OrderRequest,
    ) -> Result<()> {
        // Calculate portfolio volatility after adding the order
        let mut portfolio_variance = 0.0;
        let mut total_value = 0.0;
        
        // Current positions contribution to volatility
        for position in current_positions.values() {
            if let Some(volatility_data) = self.volatility_data.get(&position.symbol) {
                let position_value = position.size.abs() * position.current_price;
                let weight = position_value / self.portfolio_value;
                portfolio_variance += (weight * volatility_data.daily_volatility / 100.0).powi(2);
                total_value += position_value;
            }
        }
        
        // Add new order contribution
        if let Some(volatility_data) = self.volatility_data.get(&order.symbol) {
            let order_value = match order.price {
                Some(price) => order.quantity * price,
                None => {
                    if let Some(position) = current_positions.get(&order.symbol) {
                        order.quantity * position.current_price
                    } else {
                        return Ok(()); // Can't validate without price
                    }
                }
            };
            
            let new_total_value = total_value + order_value;
            let weight = order_value / new_total_value;
            portfolio_variance += (weight * volatility_data.daily_volatility / 100.0).powi(2);
        }
        
        let portfolio_volatility = portfolio_variance.sqrt() * 100.0; // Convert to percentage
        
        if portfolio_volatility > self.config.max_portfolio_volatility_pct {
            return Err(RiskError::VolatilityLimitExceeded {
                current_volatility_pct: portfolio_volatility,
                max_volatility_pct: self.config.max_portfolio_volatility_pct,
            });
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    
    fn create_test_position(symbol: &str, size: f64, entry_price: f64, current_price: f64) -> Position {
        Position {
            symbol: symbol.to_string(),
            size,
            entry_price,
            current_price,
            unrealized_pnl: (current_price - entry_price) * size,
            realized_pnl: 0.0,
            funding_pnl: 0.0,
            timestamp: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
        }
    }
    
    fn create_test_order(symbol: &str, side: OrderSide, quantity: f64, price: Option<f64>) -> OrderRequest {
        OrderRequest {
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Limit,
            quantity,
            price,
            reduce_only: false,
            time_in_force: crate::trading_mode_impl::TimeInForce::GoodTillCancel,
        }
    }
    
    #[test]
    fn test_position_size_validation() {
        let config = RiskConfig {
            max_position_size_pct: 0.1,  // 10% of portfolio
            max_daily_loss_pct: 0.02,    // 2% max daily loss
            stop_loss_pct: 0.05,         // 5% stop loss
            take_profit_pct: 0.1,        // 10% take profit
            max_leverage: 3.0,           // 3x max leverage
        };
        
        let portfolio_value = 10000.0;
        let mut risk_manager = RiskManager::new(config, portfolio_value);
        
        let mut positions = HashMap::new();
        
        // Test valid order within position size limit
        let order = create_test_order("BTC", OrderSide::Buy, 0.1, Some(9000.0));
        // Order value: 0.1 * 9000 = 900, which is < 10% of 10000 (1000)
        assert!(risk_manager.validate_order(&order, &positions).is_ok());
        
        // Test order exceeding position size limit
        let order = create_test_order("BTC", OrderSide::Buy, 0.2, Some(9000.0));
        // Order value: 0.2 * 9000 = 1800, which is > 10% of 10000 (1000)
        assert!(risk_manager.validate_order(&order, &positions).is_err());
        
        // Test with existing position
        positions.insert(
            "BTC".to_string(),
            create_test_position("BTC", 0.05, 8000.0, 9000.0)
        );
        
        // Test valid order with existing position
        let order = create_test_order("BTC", OrderSide::Buy, 0.05, Some(9000.0));
        // Existing position value: 0.05 * 9000 = 450
        // Order value: 0.05 * 9000 = 450
        // Total: 900, which is < 10% of 10000 (1000)
        assert!(risk_manager.validate_order(&order, &positions).is_ok());
        
        // Test order exceeding position size limit with existing position
        let order = create_test_order("BTC", OrderSide::Buy, 0.07, Some(9000.0));
        // Existing position value: 0.05 * 9000 = 450
        // Order value: 0.07 * 9000 = 630
        // Total: 1080, which is > 10% of 10000 (1000)
        assert!(risk_manager.validate_order(&order, &positions).is_err());
    }
    
    #[test]
    fn test_leverage_validation() {
        let config = RiskConfig {
            max_position_size_pct: 0.5,  // 50% of portfolio (high to test leverage)
            max_daily_loss_pct: 0.02,    // 2% max daily loss
            stop_loss_pct: 0.05,         // 5% stop loss
            take_profit_pct: 0.1,        // 10% take profit
            max_leverage: 2.0,           // 2x max leverage
        };
        
        let portfolio_value = 10000.0;
        let mut risk_manager = RiskManager::new(config, portfolio_value);
        
        let mut positions = HashMap::new();
        positions.insert(
            "ETH".to_string(),
            create_test_position("ETH", 2.0, 1500.0, 1600.0)
        );
        // ETH position value: 2.0 * 1600 = 3200
        
        // Test valid order within leverage limit
        let order = create_test_order("BTC", OrderSide::Buy, 0.1, Some(9000.0));
        // Order value: 0.1 * 9000 = 900
        // Total position value: 3200 + 900 = 4100
        // Leverage: 4100 / 10000 = 0.41, which is < 2.0
        assert!(risk_manager.validate_order(&order, &positions).is_ok());
        
        // Test order exceeding leverage limit
        let order = create_test_order("BTC", OrderSide::Buy, 2.0, Some(9000.0));
        // Order value: 2.0 * 9000 = 18000
        // Total position value: 3200 + 18000 = 21200
        // Leverage: 21200 / 10000 = 2.12, which is > 2.0
        assert!(risk_manager.validate_order(&order, &positions).is_err());
    }
    
    #[test]
    fn test_daily_loss_limit() {
        let config = RiskConfig {
            max_position_size_pct: 0.1,  // 10% of portfolio
            max_daily_loss_pct: 2.0,     // 2% max daily loss
            stop_loss_pct: 0.05,         // 5% stop loss
            take_profit_pct: 0.1,        // 10% take profit
            max_leverage: 3.0,           // 3x max leverage
        };
        
        let portfolio_value = 10000.0;
        let mut risk_manager = RiskManager::new(config, portfolio_value);
        
        // Update portfolio value with small loss (1%)
        assert!(risk_manager.update_portfolio_value(9900.0, -100.0).is_ok());
        
        // Verify daily loss is tracked correctly
        let (daily_loss_pct, _, _) = risk_manager.daily_risk_metrics();
        assert_eq!(daily_loss_pct, 1.0);
        
        // Update portfolio value with loss exceeding daily limit (3% total)
        assert!(risk_manager.update_portfolio_value(9700.0, -200.0).is_err());
        
        // Verify trading should be stopped
        assert!(risk_manager.should_stop_trading());
    }
    
    #[test]
    fn test_stop_loss_generation() {
        let config = RiskConfig {
            max_position_size_pct: 0.1,  // 10% of portfolio
            max_daily_loss_pct: 2.0,     // 2% max daily loss
            stop_loss_pct: 0.05,         // 5% stop loss
            take_profit_pct: 0.1,        // 10% take profit
            max_leverage: 3.0,           // 3x max leverage
        };
        
        let portfolio_value = 10000.0;
        let risk_manager = RiskManager::new(config, portfolio_value);
        
        // Test stop loss for long position
        let long_position = create_test_position("BTC", 0.1, 10000.0, 10000.0);
        let stop_loss = risk_manager.generate_stop_loss(&long_position, "order1").unwrap();
        
        assert_eq!(stop_loss.symbol, "BTC");
        assert!(matches!(stop_loss.side, OrderSide::Sell));
        assert!(matches!(stop_loss.order_type, OrderType::StopMarket));
        assert_eq!(stop_loss.quantity, 0.1);
        assert_eq!(stop_loss.trigger_price, 9500.0); // 5% below entry price
        
        // Test stop loss for short position
        let short_position = create_test_position("BTC", -0.1, 10000.0, 10000.0);
        let stop_loss = risk_manager.generate_stop_loss(&short_position, "order2").unwrap();
        
        assert_eq!(stop_loss.symbol, "BTC");
        assert!(matches!(stop_loss.side, OrderSide::Buy));
        assert!(matches!(stop_loss.order_type, OrderType::StopMarket));
        assert_eq!(stop_loss.quantity, 0.1);
        assert_eq!(stop_loss.trigger_price, 10500.0); // 5% above entry price
    }
    
    #[test]
    fn test_take_profit_generation() {
        let config = RiskConfig {
            max_position_size_pct: 0.1,  // 10% of portfolio
            max_daily_loss_pct: 2.0,     // 2% max daily loss
            stop_loss_pct: 0.05,         // 5% stop loss
            take_profit_pct: 0.1,        // 10% take profit
            max_leverage: 3.0,           // 3x max leverage
        };
        
        let portfolio_value = 10000.0;
        let risk_manager = RiskManager::new(config, portfolio_value);
        
        // Test take profit for long position
        let long_position = create_test_position("BTC", 0.1, 10000.0, 10000.0);
        let take_profit = risk_manager.generate_take_profit(&long_position, "order1").unwrap();
        
        assert_eq!(take_profit.symbol, "BTC");
        assert!(matches!(take_profit.side, OrderSide::Sell));
        assert!(matches!(take_profit.order_type, OrderType::TakeProfitMarket));
        assert_eq!(take_profit.quantity, 0.1);
        assert_eq!(take_profit.trigger_price, 11000.0); // 10% above entry price
        
        // Test take profit for short position
        let short_position = create_test_position("BTC", -0.1, 10000.0, 10000.0);
        let take_profit = risk_manager.generate_take_profit(&short_position, "order2").unwrap();
        
        assert_eq!(take_profit.symbol, "BTC");
        assert!(matches!(take_profit.side, OrderSide::Buy));
        assert!(matches!(take_profit.order_type, OrderType::TakeProfitMarket));
        assert_eq!(take_profit.quantity, 0.1);
        assert_eq!(take_profit.trigger_price, 9000.0); // 10% below entry price
    }
    
    #[test]
    fn test_risk_orders_triggering() {
        let config = RiskConfig {
            max_position_size_pct: 0.1,
            max_daily_loss_pct: 2.0,
            stop_loss_pct: 0.05,
            take_profit_pct: 0.1,
            max_leverage: 3.0,
        };
        
        let portfolio_value = 10000.0;
        let mut risk_manager = RiskManager::new(config, portfolio_value);
        
        // Create and register a stop loss order
        let long_position = create_test_position("BTC", 0.1, 10000.0, 10000.0);
        let stop_loss = risk_manager.generate_stop_loss(&long_position, "order1").unwrap();
        risk_manager.register_stop_loss(stop_loss);
        
        // Create and register a take profit order
        let take_profit = risk_manager.generate_take_profit(&long_position, "order1").unwrap();
        risk_manager.register_take_profit(take_profit);
        
        // Test no orders triggered at current price
        let mut current_prices = HashMap::new();
        current_prices.insert("BTC".to_string(), 10000.0);
        let triggered = risk_manager.check_risk_orders(&current_prices);
        assert_eq!(triggered.len(), 0);
        
        // Test stop loss triggered
        current_prices.insert("BTC".to_string(), 9400.0); // Below stop loss price
        let triggered = risk_manager.check_risk_orders(&current_prices);
        assert_eq!(triggered.len(), 1);
        assert!(triggered[0].is_stop_loss);
        
        // Register new orders
        let long_position = create_test_position("BTC", 0.1, 10000.0, 10000.0);
        let stop_loss = risk_manager.generate_stop_loss(&long_position, "order2").unwrap();
        risk_manager.register_stop_loss(stop_loss);
        let take_profit = risk_manager.generate_take_profit(&long_position, "order2").unwrap();
        risk_manager.register_take_profit(take_profit);
        
        // Test take profit triggered
        current_prices.insert("BTC".to_string(), 11100.0); // Above take profit price
        let triggered = risk_manager.check_risk_orders(&current_prices);
        assert_eq!(triggered.len(), 1);
        assert!(triggered[0].is_take_profit);
    }
    
    #[test]
    fn test_emergency_stop() {
        let config = RiskConfig::default();
        let portfolio_value = 10000.0;
        let mut risk_manager = RiskManager::new(config, portfolio_value);
        
        // Initially, emergency stop should be false
        assert!(!risk_manager.should_stop_trading());
        
        // Activate emergency stop
        risk_manager.activate_emergency_stop();
        assert!(risk_manager.should_stop_trading());
        
        // Orders should be rejected when emergency stop is active
        let positions = HashMap::new();
        let order = create_test_order("BTC", OrderSide::Buy, 0.1, Some(10000.0));
        assert!(risk_manager.validate_order(&order, &positions).is_err());
        
        // Deactivate emergency stop
        risk_manager.deactivate_emergency_stop();
        assert!(!risk_manager.should_stop_trading());
        
        // Orders should be accepted again
        assert!(risk_manager.validate_order(&order, &positions).is_ok());
    }
    /// Get the asset class for a symbol
    pub fn get_asset_class(&self, symbol: &str) -> Option<&AssetClass> {
        self.asset_classes.get(symbol)
    }
    
    /// Set the asset class for a symbol
    pub fn set_asset_class(&mut self, symbol: String, asset_class: AssetClass) {
        self.asset_classes.insert(symbol, asset_class);
    }
    
    /// Calculate the concentration of an asset class after a potential order
    fn calculate_concentration_after_order(
        &self,
        current_positions: &HashMap<String, Position>,
        order: &OrderRequest,
        asset_class: &AssetClass
    ) -> f64 {
        // Calculate current concentration by asset class
        let mut asset_class_values = HashMap::new();
        let mut total_position_value = 0.0;
        
        // Add current positions
        for (symbol, position) in current_positions {
            let position_value = position.size.abs() * position.current_price;
            total_position_value += position_value;
            
            if let Some(class) = self.get_asset_class(symbol) {
                *asset_class_values.entry(class).or_insert(0.0) += position_value;
            }
        }
        
        // Calculate order value
        let order_value = match order.price {
            Some(price) => order.quantity * price,
            None => {
                if let Some(position) = current_positions.get(&order.symbol) {
                    order.quantity * position.current_price
                } else {
                    // If we can't determine the price, assume zero (conservative)
                    0.0
                }
            }
        };
        
        // Update total position value
        total_position_value += order_value;
        
        // Update asset class value
        *asset_class_values.entry(asset_class).or_insert(0.0) += order_value;
        
        // Calculate concentration
        if total_position_value > 0.0 {
            asset_class_values.get(asset_class).unwrap_or(&0.0) / total_position_value
        } else {
            0.0
        }
    }
    
    /// Update volatility data for a symbol
    pub fn update_volatility_data(&mut self, symbol: String, price_history: Vec<f64>) {
        if price_history.len() < 30 {
            warn!("Insufficient price history for volatility calculation for {}", symbol);
            return;
        }
        
        // Calculate daily returns
        let mut daily_returns = Vec::with_capacity(price_history.len() - 1);
        for i in 1..price_history.len() {
            let daily_return = (price_history[i] - price_history[i-1]) / price_history[i-1];
            daily_returns.push(daily_return);
        }
        
        // Calculate daily volatility (standard deviation of returns)
        let mean_return = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        let variance = daily_returns.iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>() / daily_returns.len() as f64;
        let daily_volatility = variance.sqrt() * 100.0; // Convert to percentage
        
        // Calculate weekly volatility (approximate)
        let weekly_volatility = daily_volatility * (5.0_f64).sqrt();
        
        // Calculate monthly volatility (approximate)
        let monthly_volatility = daily_volatility * (21.0_f64).sqrt();
        
        // Update volatility data
        self.volatility_data.insert(symbol.clone(), VolatilityData {
            symbol,
            daily_volatility,
            weekly_volatility,
            monthly_volatility,
            price_history,
            last_update: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
        });
        
        // Update portfolio metrics
        self.update_portfolio_metrics();
    }
    
    /// Calculate volatility-adjusted position size
    fn calculate_volatility_adjusted_position_size(&self, symbol: &str, base_position_size: f64) -> f64 {
        if let Some(volatility_data) = self.volatility_data.get(symbol) {
            // Use daily volatility for adjustment
            // Higher volatility -> smaller position size
            let volatility_factor = 1.0 - (self.config.volatility_sizing_factor * 
                                          (volatility_data.daily_volatility / 100.0));
            
            // Ensure factor is between 0.1 and 1.0
            let adjusted_factor = volatility_factor.max(0.1).min(1.0);
            
            base_position_size * adjusted_factor
        } else {
            // If no volatility data, return the base position size
            base_position_size
        }
    }
    
    /// Update correlation data between two symbols
    pub fn update_correlation_data(&mut self, symbol1: String, symbol2: String, price_history1: &[f64], price_history2: &[f64]) {
        if price_history1.len() < 30 || price_history2.len() < 30 || price_history1.len() != price_history2.len() {
            warn!("Insufficient or mismatched price history for correlation calculation");
            return;
        }
        
        // Calculate returns
        let mut returns1 = Vec::with_capacity(price_history1.len() - 1);
        let mut returns2 = Vec::with_capacity(price_history2.len() - 1);
        
        for i in 1..price_history1.len() {
            let return1 = (price_history1[i] - price_history1[i-1]) / price_history1[i-1];
            let return2 = (price_history2[i] - price_history2[i-1]) / price_history2[i-1];
            returns1.push(return1);
            returns2.push(return2);
        }
        
        // Calculate correlation coefficient
        let mean1 = returns1.iter().sum::<f64>() / returns1.len() as f64;
        let mean2 = returns2.iter().sum::<f64>() / returns2.len() as f64;
        
        let mut numerator = 0.0;
        let mut denom1 = 0.0;
        let mut denom2 = 0.0;
        
        for i in 0..returns1.len() {
            let diff1 = returns1[i] - mean1;
            let diff2 = returns2[i] - mean2;
            numerator += diff1 * diff2;
            denom1 += diff1 * diff1;
            denom2 += diff2 * diff2;
        }
        
        let correlation = if denom1 > 0.0 && denom2 > 0.0 {
            numerator / (denom1.sqrt() * denom2.sqrt())
        } else {
            0.0
        };
        
        // Store correlation data (both directions)
        let correlation_data = CorrelationData {
            symbol1: symbol1.clone(),
            symbol2: symbol2.clone(),
            correlation,
            last_update: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
        };
        
        self.correlation_data.insert((symbol1.clone(), symbol2.clone()), correlation_data.clone());
        self.correlation_data.insert((symbol2, symbol1), CorrelationData {
            symbol1: correlation_data.symbol2,
            symbol2: correlation_data.symbol1,
            correlation: correlation_data.correlation,
            last_update: correlation_data.last_update,
        });
    }
    
    /// Get correlation between two symbols
    pub fn get_correlation(&self, symbol1: &str, symbol2: &str) -> Option<f64> {
        self.correlation_data.get(&(symbol1.to_string(), symbol2.to_string()))
            .map(|data| data.correlation)
    }
    
    /// Validate correlation limits for a new order
    fn validate_correlation_limits(&self, current_positions: &HashMap<String, Position>, order: &OrderRequest) -> Result<()> {
        // Skip validation if no correlation data or no existing positions
        if self.correlation_data.is_empty() || current_positions.is_empty() {
            return Ok(());
        }
        
        // Check correlation with existing positions
        for (symbol, position) in current_positions {
            // Skip the same symbol or positions with zero size
            if symbol == &order.symbol || position.size == 0.0 {
                continue;
            }
            
            // Check if we have correlation data
            if let Some(correlation) = self.get_correlation(&order.symbol, symbol) {
                // Only check for high positive correlation if positions are in the same direction
                let same_direction = (order.side == OrderSide::Buy && position.size > 0.0) ||
                                    (order.side == OrderSide::Sell && position.size < 0.0);
                
                if same_direction && correlation.abs() > self.config.max_position_correlation {
                    return Err(RiskError::CorrelationLimitExceeded {
                        symbol1: order.symbol.clone(),
                        symbol2: symbol.clone(),
                        correlation,
                        max_correlation: self.config.max_position_correlation,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Update portfolio metrics
    pub fn update_portfolio_metrics(&mut self) {
        // Calculate portfolio volatility if we have volatility data
        if !self.volatility_data.is_empty() {
            self.calculate_portfolio_volatility();
        }
        
        // Update drawdown
        self.calculate_drawdown();
        
        // Update Value at Risk (VaR)
        self.calculate_value_at_risk();
    }
    
    /// Calculate portfolio volatility
    fn calculate_portfolio_volatility(&mut self) {
        // This is a simplified portfolio volatility calculation
        // In a real implementation, we would use a covariance matrix
        
        // Get total portfolio value
        let total_value = self.portfolio_value;
        if total_value <= 0.0 {
            self.portfolio_metrics.volatility = 0.0;
            return;
        }
        
        // Calculate weighted volatility
        let mut weighted_volatility = 0.0;
        let mut total_weighted_value = 0.0;
        
        for (symbol, volatility_data) in &self.volatility_data {
            // Assume we have a position in this asset
            // In a real implementation, we would check actual positions
            let weight = 1.0 / self.volatility_data.len() as f64;
            weighted_volatility += volatility_data.daily_volatility * weight;
            total_weighted_value += weight;
        }
        
        // Normalize
        if total_weighted_value > 0.0 {
            self.portfolio_metrics.volatility = weighted_volatility / total_weighted_value;
        } else {
            self.portfolio_metrics.volatility = 0.0;
        }
    }
    
    /// Calculate drawdown
    fn calculate_drawdown(&mut self) {
        if self.historical_portfolio_values.is_empty() {
            self.portfolio_metrics.max_drawdown = 0.0;
            return;
        }
        
        // Find peak value
        let mut peak_value = self.historical_portfolio_values[0].1;
        let mut max_drawdown = 0.0;
        
        for &(_, value) in &self.historical_portfolio_values {
            if value > peak_value {
                peak_value = value;
            }
            
            let drawdown = if peak_value > 0.0 {
                (peak_value - value) / peak_value
            } else {
                0.0
            };
            
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
        
        self.portfolio_metrics.max_drawdown = max_drawdown;
    }
    
    /// Calculate Value at Risk (VaR)
    fn calculate_value_at_risk(&mut self) {
        // This is a simplified VaR calculation using historical simulation
        // In a real implementation, we would use more sophisticated methods
        
        if self.historical_portfolio_values.len() < 30 {
            self.portfolio_metrics.var_95 = 0.0;
            self.portfolio_metrics.var_99 = 0.0;
            return;
        }
        
        // Calculate daily returns
        let mut daily_returns = Vec::with_capacity(self.historical_portfolio_values.len() - 1);
        for i in 1..self.historical_portfolio_values.len() {
            let prev_value = self.historical_portfolio_values[i-1].1;
            let curr_value = self.historical_portfolio_values[i].1;
            
            if prev_value > 0.0 {
                let daily_return = (curr_value - prev_value) / prev_value;
                daily_returns.push(daily_return);
            }
        }
        
        // Sort returns in ascending order
        daily_returns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        // Calculate VaR at 95% confidence
        let index_95 = (daily_returns.len() as f64 * 0.05).floor() as usize;
        if index_95 < daily_returns.len() {
            self.portfolio_metrics.var_95 = -daily_returns[index_95] * self.portfolio_value;
        }
        
        // Calculate VaR at 99% confidence
        let index_99 = (daily_returns.len() as f64 * 0.01).floor() as usize;
        if index_99 < daily_returns.len() {
            self.portfolio_metrics.var_99 = -daily_returns[index_99] * self.portfolio_value;
        }
    }
    
    /// Validate portfolio volatility limits
    fn validate_portfolio_volatility(&self, current_positions: &HashMap<String, Position>, order: &OrderRequest) -> Result<()> {
        // Skip validation if we don't have enough volatility data
        if self.volatility_data.is_empty() {
            return Ok(());
        }
        
        // Check if adding this position would exceed portfolio volatility limits
        // This is a simplified check - in a real implementation, we would recalculate portfolio volatility
        
        if let Some(volatility_data) = self.volatility_data.get(&order.symbol) {
            // If the asset is more volatile than our limit and it's a significant position
            let order_value = match order.price {
                Some(price) => order.quantity * price,
                None => {
                    if let Some(position) = current_positions.get(&order.symbol) {
                        order.quantity * position.current_price
                    } else {
                        0.0
                    }
                }
            };
            
            let position_weight = order_value / self.portfolio_value;
            
            // If this is a significant position in a highly volatile asset
            if position_weight > 0.1 && volatility_data.daily_volatility > self.config.max_portfolio_volatility_pct {
                return Err(RiskError::VolatilityLimitExceeded {
                    current_volatility_pct: volatility_data.daily_volatility,
                    max_volatility_pct: self.config.max_portfolio_volatility_pct,
                });
            }
            
            // If current portfolio volatility is already near the limit
            if self.portfolio_metrics.volatility > self.config.max_portfolio_volatility_pct * 0.9 {
                // And this asset is more volatile than the portfolio
                if volatility_data.daily_volatility > self.portfolio_metrics.volatility {
                    return Err(RiskError::VolatilityLimitExceeded {
                        current_volatility_pct: self.portfolio_metrics.volatility,
                        max_volatility_pct: self.config.max_portfolio_volatility_pct,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Update portfolio value and track historical values
    pub fn update_portfolio_value_with_history(&mut self, new_value: f64, realized_pnl_delta: f64) -> Result<()> {
        // Update regular portfolio value
        let result = self.update_portfolio_value(new_value, realized_pnl_delta);
        
        // Add to historical values
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        self.historical_portfolio_values.push((now, new_value));
        
        // Limit history size to prevent memory issues
        if self.historical_portfolio_values.len() > 1000 {
            self.historical_portfolio_values.remove(0);
        }
        
        // Update portfolio metrics
        self.update_portfolio_metrics();
        
        // Check drawdown limit
        if self.portfolio_metrics.max_drawdown > self.config.max_drawdown_pct {
            warn!(
                "Maximum drawdown limit reached: {:.2}% exceeds {:.2}%",
                self.portfolio_metrics.max_drawdown * 100.0,
                self.config.max_drawdown_pct * 100.0
            );
            
            // Activate emergency stop
            self.activate_emergency_stop();
            
            return Err(RiskError::DrawdownLimitExceeded {
                current_drawdown_pct: self.portfolio_metrics.max_drawdown * 100.0,
                max_drawdown_pct: self.config.max_drawdown_pct * 100.0,
            });
        }
        
        result
    }
    
    /// Get current portfolio metrics
    pub fn get_portfolio_metrics(&self) -> &PortfolioMetrics {
        &self.portfolio_metrics
    }
    
    /// Get volatility data for a symbol
    pub fn get_volatility_data(&self, symbol: &str) -> Option<&VolatilityData> {
        self.volatility_data.get(symbol)
    }
    
    /// Get all volatility data
    pub fn get_all_volatility_data(&self) -> &HashMap<String, VolatilityData> {
        &self.volatility_data
    }
    
    /// Get all correlation data
    pub fn get_all_correlation_data(&self) -> &HashMap<(String, String), CorrelationData> {
        &self.correlation_data
    }

    /// Calculate volatility-adjusted position size
    pub fn calculate_volatility_adjusted_position_size(&self, symbol: &str, base_size: f64) -> f64 {
        if let Some(volatility_data) = self.volatility_data.get(symbol) {
            // Adjust position size based on volatility
            let volatility_factor = 1.0 / (1.0 + volatility_data.daily_volatility);
            base_size * volatility_factor
        } else {
            base_size
        }
    }

    /// Get asset class for a symbol (placeholder implementation)
    pub fn get_asset_class(&self, _symbol: &str) -> Option<String> {
        // Placeholder - in real implementation, this would classify assets
        Some("crypto".to_string())
    }

    /// Calculate concentration after order
    pub fn calculate_concentration_after_order(
        &self, 
        current_positions: &HashMap<String, Position>, 
        order: &OrderRequest, 
        asset_class: String
    ) -> f64 {
        // Calculate current concentration for this asset class
        let current_class_value: f64 = current_positions.values()
            .filter(|p| self.get_asset_class(&p.symbol).as_deref() == Some(&asset_class))
            .map(|p| p.size.abs() * p.current_price)
            .sum();
        
        // Add the new order value if it's the same asset class
        let order_value = if self.get_asset_class(&order.symbol).as_deref() == Some(&asset_class) {
            order.quantity.abs() * order.price.unwrap_or(0.0)
        } else {
            0.0
        };
        
        let total_class_value = current_class_value + order_value;
        
        // Return concentration as percentage of portfolio
        if self.portfolio_value > 0.0 {
            total_class_value / self.portfolio_value
        } else {
            0.0
        }
    }

    /// Validate correlation limits
    pub fn validate_correlation_limits(
        &self, 
        current_positions: &HashMap<String, Position>, 
        order: &OrderRequest
    ) -> Result<()> {
        // Simplified implementation - check if adding this position would create high correlation
        for position in current_positions.values() {
            if let Some(correlation_data) = self.correlation_data.get(&(position.symbol.clone(), order.symbol.clone())) {
                if correlation_data.correlation.abs() > 0.8 {
                    return Err(RiskError::CorrelationLimitExceeded {
                        symbol1: position.symbol.clone(),
                        symbol2: order.symbol.clone(),
                        correlation: correlation_data.correlation,
                        max_correlation: 0.8,
                    });
                }
            }
        }
        Ok(())
    }

    /// Validate portfolio volatility
    pub fn validate_portfolio_volatility(
        &self, 
        _current_positions: &HashMap<String, Position>, 
        _order: &OrderRequest
    ) -> Result<()> {
        // Check if current portfolio volatility is within limits
        if self.portfolio_metrics.volatility > self.config.max_portfolio_volatility_pct {
            return Err(RiskError::VolatilityLimitExceeded {
                current_volatility_pct: self.portfolio_metrics.volatility,
                max_volatility_pct: self.config.max_portfolio_volatility_pct,
            });
        }
        Ok(())
    }
}