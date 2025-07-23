//! # Unified Data Structures Implementation
//!
//! This module provides the implementation of unified data structures that work across all trading modes
//! (backtest, paper trading, live trading) to ensure consistent strategy execution
//! and seamless transitions between modes.

use std::collections::HashMap;
use chrono::{DateTime, FixedOffset};

/// Position information across all trading modes
#[derive(Debug, Clone)]
pub struct Position {
    /// Symbol/ticker of the asset
    pub symbol: String,
    
    /// Position size (positive for long, negative for short)
    pub size: f64,
    
    /// Entry price
    pub entry_price: f64,
    
    /// Current price
    pub current_price: f64,
    
    /// Unrealized profit and loss
    pub unrealized_pnl: f64,
    
    /// Realized profit and loss
    pub realized_pnl: f64,
    
    /// Funding profit and loss (for perpetual futures)
    pub funding_pnl: f64,
    
    /// Position timestamp
    pub timestamp: DateTime<FixedOffset>,
    
    /// Leverage used for this position
    pub leverage: f64,
    
    /// Liquidation price (if applicable)
    pub liquidation_price: Option<f64>,
    
    /// Position margin (if applicable)
    pub margin: Option<f64>,
    
    /// Additional position metadata
    pub metadata: HashMap<String, String>,
}

impl Position {
    /// Create a new position
    pub fn new(
        symbol: &str,
        size: f64,
        entry_price: f64,
        current_price: f64,
        timestamp: DateTime<FixedOffset>,
    ) -> Self {
        let unrealized_pnl = if size != 0.0 {
            size * (current_price - entry_price)
        } else {
            0.0
        };
        
        Self {
            symbol: symbol.to_string(),
            size,
            entry_price,
            current_price,
            unrealized_pnl,
            realized_pnl: 0.0,
            funding_pnl: 0.0,
            timestamp,
            leverage: 1.0,
            liquidation_price: None,
            margin: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Update the position with a new price
    pub fn update_price(&mut self, price: f64) {
        self.current_price = price;
        if self.size != 0.0 {
            self.unrealized_pnl = self.size * (price - self.entry_price);
        }
    }
    
    /// Apply a funding payment to the position
    pub fn apply_funding_payment(&mut self, payment: f64) {
        self.funding_pnl += payment;
    }
    
    /// Get the total PnL (realized + unrealized + funding)
    pub fn total_pnl(&self) -> f64 {
        self.realized_pnl + self.unrealized_pnl + self.funding_pnl
    }
    
    /// Get the position notional value
    pub fn notional_value(&self) -> f64 {
        self.size.abs() * self.current_price
    }
    
    /// Check if the position is long
    pub fn is_long(&self) -> bool {
        self.size > 0.0
    }
    
    /// Check if the position is short
    pub fn is_short(&self) -> bool {
        self.size < 0.0
    }
    
    /// Check if the position is flat (no position)
    pub fn is_flat(&self) -> bool {
        self.size == 0.0
    }
}

/// Order side (buy/sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    /// Buy order
    Buy,
    
    /// Sell order
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "Buy"),
            OrderSide::Sell => write!(f, "Sell"),
        }
    }
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    /// Market order
    Market,
    
    /// Limit order
    Limit,
    
    /// Stop market order
    StopMarket,
    
    /// Stop limit order
    StopLimit,
    
    /// Take profit market order
    TakeProfitMarket,
    
    /// Take profit limit order
    TakeProfitLimit,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::Market => write!(f, "Market"),
            OrderType::Limit => write!(f, "Limit"),
            OrderType::StopMarket => write!(f, "StopMarket"),
            OrderType::StopLimit => write!(f, "StopLimit"),
            OrderType::TakeProfitMarket => write!(f, "TakeProfitMarket"),
            OrderType::TakeProfitLimit => write!(f, "TakeProfitLimit"),
        }
    }
}

/// Time in force policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInForce {
    /// Good till cancelled
    GoodTillCancel,
    
    /// Immediate or cancel
    ImmediateOrCancel,
    
    /// Fill or kill
    FillOrKill,
    
    /// Good till date
    GoodTillDate,
}

impl std::fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeInForce::GoodTillCancel => write!(f, "GoodTillCancel"),
            TimeInForce::ImmediateOrCancel => write!(f, "ImmediateOrCancel"),
            TimeInForce::FillOrKill => write!(f, "FillOrKill"),
            TimeInForce::GoodTillDate => write!(f, "GoodTillDate"),
        }
    }
}

/// Order request across all trading modes
#[derive(Debug, Clone)]
pub struct OrderRequest {
    /// Symbol/ticker of the asset
    pub symbol: String,
    
    /// Order side (buy/sell)
    pub side: OrderSide,
    
    /// Order type (market/limit/etc)
    pub order_type: OrderType,
    
    /// Order quantity
    pub quantity: f64,
    
    /// Order price (for limit orders)
    pub price: Option<f64>,
    
    /// Whether this order reduces position only
    pub reduce_only: bool,
    
    /// Time in force policy
    pub time_in_force: TimeInForce,
    
    /// Stop price (for stop orders)
    pub stop_price: Option<f64>,
    
    /// Client order ID (if any)
    pub client_order_id: Option<String>,
    
    /// Additional order parameters
    pub parameters: HashMap<String, String>,
}

impl OrderRequest {
    /// Create a new market order
    pub fn market(symbol: &str, side: OrderSide, quantity: f64) -> Self {
        Self {
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Market,
            quantity,
            price: None,
            reduce_only: false,
            time_in_force: TimeInForce::GoodTillCancel,
            stop_price: None,
            client_order_id: None,
            parameters: HashMap::new(),
        }
    }
    
    /// Create a new limit order
    pub fn limit(symbol: &str, side: OrderSide, quantity: f64, price: f64) -> Self {
        Self {
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Limit,
            quantity,
            price: Some(price),
            reduce_only: false,
            time_in_force: TimeInForce::GoodTillCancel,
            stop_price: None,
            client_order_id: None,
            parameters: HashMap::new(),
        }
    }
    
    /// Set the order as reduce-only
    pub fn reduce_only(mut self) -> Self {
        self.reduce_only = true;
        self
    }
    
    /// Set the time in force policy
    pub fn with_time_in_force(mut self, time_in_force: TimeInForce) -> Self {
        self.time_in_force = time_in_force;
        self
    }
    
    /// Set the client order ID
    pub fn with_client_order_id(mut self, client_order_id: &str) -> Self {
        self.client_order_id = Some(client_order_id.to_string());
        self
    }
    
    /// Add a parameter to the order
    pub fn with_parameter(mut self, key: &str, value: &str) -> Self {
        self.parameters.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Validate the order request
    pub fn validate(&self) -> Result<(), String> {
        // Check for positive quantity
        if self.quantity <= 0.0 {
            return Err("Order quantity must be positive".to_string());
        }
        
        // Check for price on limit orders
        if matches!(self.order_type, OrderType::Limit | OrderType::StopLimit | OrderType::TakeProfitLimit) 
            && self.price.is_none() {
            return Err(format!("Price is required for {} orders", self.order_type));
        }
        
        // Check for stop price on stop orders
        if matches!(self.order_type, OrderType::StopMarket | OrderType::StopLimit) 
            && self.stop_price.is_none() {
            return Err(format!("Stop price is required for {} orders", self.order_type));
        }
        
        // Check for take profit price on take profit orders
        if matches!(self.order_type, OrderType::TakeProfitMarket | OrderType::TakeProfitLimit) 
            && self.stop_price.is_none() {
            return Err(format!("Take profit price is required for {} orders", self.order_type));
        }
        
        Ok(())
    }
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    /// Order created but not yet submitted
    Created,
    
    /// Order submitted to exchange
    Submitted,
    
    /// Order partially filled
    PartiallyFilled,
    
    /// Order fully filled
    Filled,
    
    /// Order cancelled
    Cancelled,
    
    /// Order rejected
    Rejected,
    
    /// Order expired
    Expired,
}

impl std::fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderStatus::Created => write!(f, "Created"),
            OrderStatus::Submitted => write!(f, "Submitted"),
            OrderStatus::PartiallyFilled => write!(f, "PartiallyFilled"),
            OrderStatus::Filled => write!(f, "Filled"),
            OrderStatus::Cancelled => write!(f, "Cancelled"),
            OrderStatus::Rejected => write!(f, "Rejected"),
            OrderStatus::Expired => write!(f, "Expired"),
        }
    }
}

/// Order result across all trading modes
#[derive(Debug, Clone)]
pub struct OrderResult {
    /// Order ID
    pub order_id: String,
    
    /// Symbol/ticker of the asset
    pub symbol: String,
    
    /// Order side (buy/sell)
    pub side: OrderSide,
    
    /// Order type
    pub order_type: OrderType,
    
    /// Requested quantity
    pub requested_quantity: f64,
    
    /// Filled quantity
    pub filled_quantity: f64,
    
    /// Average fill price
    pub average_price: Option<f64>,
    
    /// Order status
    pub status: OrderStatus,
    
    /// Order timestamp
    pub timestamp: DateTime<FixedOffset>,
    
    /// Fees paid
    pub fees: Option<f64>,
    
    /// Error message (if any)
    pub error: Option<String>,
    
    /// Client order ID (if any)
    pub client_order_id: Option<String>,
    
    /// Additional order result data
    pub metadata: HashMap<String, String>,
}

impl OrderResult {
    /// Create a new order result
    pub fn new(
        order_id: &str,
        symbol: &str,
        side: OrderSide,
        order_type: OrderType,
        requested_quantity: f64,
        timestamp: DateTime<FixedOffset>,
    ) -> Self {
        Self {
            order_id: order_id.to_string(),
            symbol: symbol.to_string(),
            side,
            order_type,
            requested_quantity,
            filled_quantity: 0.0,
            average_price: None,
            status: OrderStatus::Created,
            timestamp,
            fees: None,
            error: None,
            client_order_id: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Check if the order is active
    pub fn is_active(&self) -> bool {
        matches!(self.status, OrderStatus::Created | OrderStatus::Submitted | OrderStatus::PartiallyFilled)
    }
    
    /// Check if the order is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.status, OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Rejected | OrderStatus::Expired)
    }
    
    /// Check if the order is filled (partially or fully)
    pub fn is_filled(&self) -> bool {
        matches!(self.status, OrderStatus::PartiallyFilled | OrderStatus::Filled)
    }
    
    /// Get the fill percentage
    pub fn fill_percentage(&self) -> f64 {
        if self.requested_quantity > 0.0 {
            self.filled_quantity / self.requested_quantity * 100.0
        } else {
            0.0
        }
    }
    
    /// Get the notional value of the filled quantity
    pub fn filled_notional(&self) -> Option<f64> {
        self.average_price.map(|price| self.filled_quantity * price)
    }
}

/// Market data structure for real-time data
#[derive(Debug, Clone)]
pub struct MarketData {
    /// Symbol/ticker of the asset
    pub symbol: String,
    
    /// Last price
    pub price: f64,
    
    /// Best bid price
    pub bid: f64,
    
    /// Best ask price
    pub ask: f64,
    
    /// Trading volume
    pub volume: f64,
    
    /// Timestamp
    pub timestamp: DateTime<FixedOffset>,
    
    /// Current funding rate (if available)
    pub funding_rate: Option<f64>,
    
    /// Next funding time (if available)
    pub next_funding_time: Option<DateTime<FixedOffset>>,
    
    /// Open interest (if available)
    pub open_interest: Option<f64>,
    
    /// Market depth (order book)
    pub depth: Option<OrderBookSnapshot>,
    
    /// Recent trades
    pub recent_trades: Option<Vec<Trade>>,
    
    /// 24-hour price change percentage
    pub price_change_24h_pct: Option<f64>,
    
    /// 24-hour high price
    pub high_24h: Option<f64>,
    
    /// 24-hour low price
    pub low_24h: Option<f64>,
    
    /// Additional market data
    pub metadata: HashMap<String, String>,
}

impl MarketData {
    /// Create a new market data instance with basic price information
    pub fn new(
        symbol: &str,
        price: f64,
        bid: f64,
        ask: f64,
        volume: f64,
        timestamp: DateTime<FixedOffset>,
    ) -> Self {
        Self {
            symbol: symbol.to_string(),
            price,
            bid,
            ask,
            volume,
            timestamp,
            funding_rate: None,
            next_funding_time: None,
            open_interest: None,
            depth: None,
            recent_trades: None,
            price_change_24h_pct: None,
            high_24h: None,
            low_24h: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Get the mid price (average of bid and ask)
    pub fn mid_price(&self) -> f64 {
        (self.bid + self.ask) / 2.0
    }
    
    /// Get the spread (ask - bid)
    pub fn spread(&self) -> f64 {
        self.ask - self.bid
    }
    
    /// Get the spread as a percentage of the mid price
    pub fn spread_percentage(&self) -> f64 {
        let mid = self.mid_price();
        if mid > 0.0 {
            self.spread() / mid * 100.0
        } else {
            0.0
        }
    }
    
    /// Add funding rate information
    pub fn with_funding_rate(
        mut self,
        funding_rate: f64,
        next_funding_time: DateTime<FixedOffset>,
    ) -> Self {
        self.funding_rate = Some(funding_rate);
        self.next_funding_time = Some(next_funding_time);
        self
    }
    
    /// Add open interest information
    pub fn with_open_interest(mut self, open_interest: f64) -> Self {
        self.open_interest = Some(open_interest);
        self
    }
    
    /// Add 24-hour statistics
    pub fn with_24h_stats(
        mut self,
        price_change_pct: f64,
        high: f64,
        low: f64,
    ) -> Self {
        self.price_change_24h_pct = Some(price_change_pct);
        self.high_24h = Some(high);
        self.low_24h = Some(low);
        self
    }
    
    /// Add a metadata field
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Order book level (price and quantity)
#[derive(Debug, Clone)]
pub struct OrderBookLevel {
    /// Price level
    pub price: f64,
    
    /// Quantity at this price level
    pub quantity: f64,
}

/// Order book snapshot
#[derive(Debug, Clone)]
pub struct OrderBookSnapshot {
    /// Bid levels (sorted by price descending)
    pub bids: Vec<OrderBookLevel>,
    
    /// Ask levels (sorted by price ascending)
    pub asks: Vec<OrderBookLevel>,
    
    /// Timestamp of the snapshot
    pub timestamp: DateTime<FixedOffset>,
}

/// Trade information
#[derive(Debug, Clone)]
pub struct Trade {
    /// Trade ID
    pub id: String,
    
    /// Trade price
    pub price: f64,
    
    /// Trade quantity
    pub quantity: f64,
    
    /// Trade timestamp
    pub timestamp: DateTime<FixedOffset>,
    
    /// Trade side (buy/sell)
    pub side: Option<OrderSide>,
}

/// Trading signal direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalDirection {
    /// Buy signal
    Buy,
    
    /// Sell signal
    Sell,
    
    /// Hold/neutral signal
    Neutral,
    
    /// Close position signal
    Close,
}

/// Trading signal
#[derive(Debug, Clone)]
pub struct Signal {
    /// Symbol/ticker of the asset
    pub symbol: String,
    
    /// Signal direction
    pub direction: SignalDirection,
    
    /// Signal strength (0.0 to 1.0)
    pub strength: f64,
    
    /// Signal timestamp
    pub timestamp: DateTime<FixedOffset>,
    
    /// Signal metadata
    pub metadata: HashMap<String, String>,
}

/// Trading strategy trait for unified strategy execution across all modes
pub trait TradingStrategy: Send + Sync {
    /// Get the strategy name
    fn name(&self) -> &str;
    
    /// Process market data and generate signals
    fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String>;
    
    /// Process order fill events
    fn on_order_fill(&mut self, fill: &OrderResult) -> Result<(), String>;
    
    /// Process funding payment events
    fn on_funding_payment(&mut self, payment: &FundingPayment) -> Result<(), String>;
    
    /// Get current strategy signals
    fn get_current_signals(&self) -> HashMap<String, Signal>;
}

/// Funding payment information
#[derive(Debug, Clone)]
pub struct FundingPayment {
    /// Symbol/ticker of the asset
    pub symbol: String,
    
    /// Funding rate
    pub rate: f64,
    
    /// Position size at funding time
    pub position_size: f64,
    
    /// Payment amount (positive for received, negative for paid)
    pub amount: f64,
    
    /// Payment timestamp
    pub timestamp: DateTime<FixedOffset>,
}

/// Trading configuration
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
    
    /// Maximum number of concurrent positions
    pub max_positions: usize,
    
    /// Maximum drawdown percentage before stopping trading
    pub max_drawdown_pct: f64,
    
    /// Whether to use trailing stop loss
    pub use_trailing_stop: bool,
    
    /// Trailing stop distance percentage
    pub trailing_stop_distance_pct: Option<f64>,
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
    
    /// Whether to use order book for slippage calculation
    pub use_order_book: bool,
    
    /// Maximum slippage percentage allowed
    pub max_slippage_pct: f64,
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
    
    /// Rate limit (requests per second)
    pub rate_limit: Option<f64>,
    
    /// Retry attempts for failed requests
    pub retry_attempts: u32,
    
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
}