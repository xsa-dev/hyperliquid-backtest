// This is a standalone test file that doesn't depend on the existing codebase
// It's used to verify our implementation of unified data structures

use std::collections::HashMap;
use chrono::{DateTime, FixedOffset, TimeZone, Utc};

// Position information across all trading modes
#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub size: f64,
    pub entry_price: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub funding_pnl: f64,
    pub timestamp: DateTime<FixedOffset>,
    pub leverage: f64,
    pub liquidation_price: Option<f64>,
    pub margin: Option<f64>,
    pub metadata: HashMap<String, String>,
}

impl Position {
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
    
    pub fn update_price(&mut self, price: f64) {
        self.current_price = price;
        if self.size != 0.0 {
            self.unrealized_pnl = self.size * (price - self.entry_price);
        }
    }
    
    pub fn apply_funding_payment(&mut self, payment: f64) {
        self.funding_pnl += payment;
    }
    
    pub fn total_pnl(&self) -> f64 {
        self.realized_pnl + self.unrealized_pnl + self.funding_pnl
    }
    
    pub fn notional_value(&self) -> f64 {
        self.size.abs() * self.current_price
    }
    
    pub fn is_long(&self) -> bool {
        self.size > 0.0
    }
    
    pub fn is_short(&self) -> bool {
        self.size < 0.0
    }
    
    pub fn is_flat(&self) -> bool {
        self.size == 0.0
    }
}

// Order side (buy/sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Market,
    Limit,
    StopMarket,
    StopLimit,
    TakeProfitMarket,
    TakeProfitLimit,
}

// Time in force policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInForce {
    GoodTillCancel,
    ImmediateOrCancel,
    FillOrKill,
    GoodTillDate,
}

// Order request across all trading modes
#[derive(Debug, Clone)]
pub struct OrderRequest {
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: f64,
    pub price: Option<f64>,
    pub reduce_only: bool,
    pub time_in_force: TimeInForce,
    pub stop_price: Option<f64>,
    pub client_order_id: Option<String>,
    pub parameters: HashMap<String, String>,
}

impl OrderRequest {
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
    
    pub fn reduce_only(mut self) -> Self {
        self.reduce_only = true;
        self
    }
    
    pub fn with_time_in_force(mut self, time_in_force: TimeInForce) -> Self {
        self.time_in_force = time_in_force;
        self
    }
    
    pub fn with_client_order_id(mut self, client_order_id: &str) -> Self {
        self.client_order_id = Some(client_order_id.to_string());
        self
    }
    
    pub fn with_parameter(mut self, key: &str, value: &str) -> Self {
        self.parameters.insert(key.to_string(), value.to_string());
        self
    }
    
    pub fn validate(&self) -> Result<(), String> {
        if self.quantity <= 0.0 {
            return Err("Order quantity must be positive".to_string());
        }
        
        if matches!(self.order_type, OrderType::Limit | OrderType::StopLimit | OrderType::TakeProfitLimit) 
            && self.price.is_none() {
            return Err("Price is required for limit orders".to_string());
        }
        
        if matches!(self.order_type, OrderType::StopMarket | OrderType::StopLimit) 
            && self.stop_price.is_none() {
            return Err("Stop price is required for stop orders".to_string());
        }
        
        Ok(())
    }
}

// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    Created,
    Submitted,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Expired,
}

// Order result across all trading modes
#[derive(Debug, Clone)]
pub struct OrderResult {
    pub order_id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub requested_quantity: f64,
    pub filled_quantity: f64,
    pub average_price: Option<f64>,
    pub status: OrderStatus,
    pub timestamp: DateTime<FixedOffset>,
    pub fees: Option<f64>,
    pub error: Option<String>,
    pub client_order_id: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl OrderResult {
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
    
    pub fn is_active(&self) -> bool {
        matches!(self.status, OrderStatus::Created | OrderStatus::Submitted | OrderStatus::PartiallyFilled)
    }
    
    pub fn is_complete(&self) -> bool {
        matches!(self.status, OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Rejected | OrderStatus::Expired)
    }
    
    pub fn is_filled(&self) -> bool {
        matches!(self.status, OrderStatus::PartiallyFilled | OrderStatus::Filled)
    }
    
    pub fn fill_percentage(&self) -> f64 {
        if self.requested_quantity > 0.0 {
            self.filled_quantity / self.requested_quantity * 100.0
        } else {
            0.0
        }
    }
    
    pub fn filled_notional(&self) -> Option<f64> {
        self.average_price.map(|price| self.filled_quantity * price)
    }
}

// Market data structure for real-time data
#[derive(Debug, Clone)]
pub struct MarketData {
    pub symbol: String,
    pub price: f64,
    pub bid: f64,
    pub ask: f64,
    pub volume: f64,
    pub timestamp: DateTime<FixedOffset>,
    pub funding_rate: Option<f64>,
    pub next_funding_time: Option<DateTime<FixedOffset>>,
    pub open_interest: Option<f64>,
    pub metadata: HashMap<String, String>,
}

impl MarketData {
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
            metadata: HashMap::new(),
        }
    }
    
    pub fn mid_price(&self) -> f64 {
        (self.bid + self.ask) / 2.0
    }
    
    pub fn spread(&self) -> f64 {
        self.ask - self.bid
    }
    
    pub fn spread_percentage(&self) -> f64 {
        let mid = self.mid_price();
        if mid > 0.0 {
            self.spread() / mid * 100.0
        } else {
            0.0
        }
    }
    
    pub fn with_funding_rate(
        mut self,
        funding_rate: f64,
        next_funding_time: DateTime<FixedOffset>,
    ) -> Self {
        self.funding_rate = Some(funding_rate);
        self.next_funding_time = Some(next_funding_time);
        self
    }
    
    pub fn with_open_interest(mut self, open_interest: f64) -> Self {
        self.open_interest = Some(open_interest);
        self
    }
    
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

// Trading configuration
#[derive(Debug, Clone)]
pub struct TradingConfig {
    pub initial_balance: f64,
    pub risk_config: Option<RiskConfig>,
    pub slippage_config: Option<SlippageConfig>,
    pub api_config: Option<ApiConfig>,
    pub parameters: HashMap<String, String>,
}

// Risk management configuration
#[derive(Debug, Clone)]
pub struct RiskConfig {
    pub max_position_size_pct: f64,
    pub max_daily_loss_pct: f64,
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub max_leverage: f64,
    pub max_positions: usize,
    pub max_drawdown_pct: f64,
    pub use_trailing_stop: bool,
    pub trailing_stop_distance_pct: Option<f64>,
}

// Slippage simulation configuration for paper trading
#[derive(Debug, Clone)]
pub struct SlippageConfig {
    pub base_slippage_pct: f64,
    pub volume_impact_factor: f64,
    pub volatility_impact_factor: f64,
    pub random_slippage_max_pct: f64,
    pub simulated_latency_ms: u64,
    pub use_order_book: bool,
    pub max_slippage_pct: f64,
}

// API configuration for live trading
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub api_key: String,
    pub api_secret: String,
    pub endpoint: String,
    pub use_testnet: bool,
    pub timeout_ms: u64,
    pub rate_limit: Option<f64>,
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_position_creation_and_methods() {
        let now = Utc::now().with_timezone(&FixedOffset::east(0));
        
        // Create a long position
        let mut position = Position::new("BTC", 1.0, 50000.0, 51000.0, now);
        
        // Test basic properties
        assert_eq!(position.symbol, "BTC");
        assert_eq!(position.size, 1.0);
        assert_eq!(position.entry_price, 50000.0);
        assert_eq!(position.current_price, 51000.0);
        assert_eq!(position.unrealized_pnl, 1000.0); // (51000 - 50000) * 1.0
        assert_eq!(position.realized_pnl, 0.0);
        assert_eq!(position.funding_pnl, 0.0);
        assert_eq!(position.timestamp, now);
        
        // Test position methods
        assert!(position.is_long());
        assert!(!position.is_short());
        assert!(!position.is_flat());
        assert_eq!(position.notional_value(), 51000.0); // 1.0 * 51000.0
        assert_eq!(position.total_pnl(), 1000.0); // unrealized + realized + funding
        
        // Update price and check PnL changes
        position.update_price(52000.0);
        assert_eq!(position.current_price, 52000.0);
        assert_eq!(position.unrealized_pnl, 2000.0); // (52000 - 50000) * 1.0
        
        // Apply funding payment
        position.apply_funding_payment(100.0);
        assert_eq!(position.funding_pnl, 100.0);
        assert_eq!(position.total_pnl(), 2100.0); // 2000 + 0 + 100
    }
    
    #[test]
    fn test_order_request_creation_and_validation() {
        // Create a market order
        let market_order = OrderRequest::market("BTC", OrderSide::Buy, 1.0);
        
        // Test basic properties
        assert_eq!(market_order.symbol, "BTC");
        assert_eq!(market_order.side, OrderSide::Buy);
        assert_eq!(market_order.order_type, OrderType::Market);
        assert_eq!(market_order.quantity, 1.0);
        assert_eq!(market_order.price, None);
        assert_eq!(market_order.reduce_only, false);
        assert_eq!(market_order.time_in_force, TimeInForce::GoodTillCancel);
        
        // Create a limit order
        let limit_order = OrderRequest::limit("ETH", OrderSide::Sell, 2.0, 3000.0)
            .reduce_only()
            .with_time_in_force(TimeInForce::FillOrKill)
            .with_client_order_id("test-order-123")
            .with_parameter("post_only", "true");
        
        // Test limit order properties
        assert_eq!(limit_order.symbol, "ETH");
        assert_eq!(limit_order.side, OrderSide::Sell);
        assert_eq!(limit_order.order_type, OrderType::Limit);
        assert_eq!(limit_order.quantity, 2.0);
        assert_eq!(limit_order.price, Some(3000.0));
        assert_eq!(limit_order.reduce_only, true);
        assert_eq!(limit_order.time_in_force, TimeInForce::FillOrKill);
        assert_eq!(limit_order.client_order_id, Some("test-order-123".to_string()));
        assert_eq!(limit_order.parameters.get("post_only"), Some(&"true".to_string()));
        
        // Test order validation
        assert!(market_order.validate().is_ok());
        assert!(limit_order.validate().is_ok());
        
        // Test validation failures
        let invalid_quantity = OrderRequest::market("BTC", OrderSide::Buy, 0.0);
        assert!(invalid_quantity.validate().is_err());
        
        let invalid_limit = OrderRequest {
            symbol: "BTC".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            quantity: 1.0,
            price: None, // Missing price for limit order
            reduce_only: false,
            time_in_force: TimeInForce::GoodTillCancel,
            stop_price: None,
            client_order_id: None,
            parameters: HashMap::new(),
        };
        assert!(invalid_limit.validate().is_err());
    }
    
    #[test]
    fn test_market_data_creation_and_methods() {
        let now = Utc::now().with_timezone(&FixedOffset::east(0));
        let next_funding = now + chrono::Duration::hours(8);
        
        // Create basic market data
        let market_data = MarketData::new(
            "BTC",
            50000.0,
            49990.0,
            50010.0,
            100.0,
            now,
        );
        
        // Test basic properties
        assert_eq!(market_data.symbol, "BTC");
        assert_eq!(market_data.price, 50000.0);
        assert_eq!(market_data.bid, 49990.0);
        assert_eq!(market_data.ask, 50010.0);
        assert_eq!(market_data.volume, 100.0);
        assert_eq!(market_data.timestamp, now);
        
        // Test calculated properties
        assert_eq!(market_data.mid_price(), 50000.0); // (49990 + 50010) / 2
        assert_eq!(market_data.spread(), 20.0); // 50010 - 49990
        assert_eq!(market_data.spread_percentage(), 0.04); // (20 / 50000) * 100
        
        // Test builder methods
        let enhanced_data = market_data
            .with_funding_rate(0.0001, next_funding)
            .with_open_interest(1000.0)
            .with_metadata("exchange", "hyperliquid");
        
        assert_eq!(enhanced_data.funding_rate, Some(0.0001));
        assert_eq!(enhanced_data.next_funding_time, Some(next_funding));
        assert_eq!(enhanced_data.open_interest, Some(1000.0));
        assert_eq!(enhanced_data.metadata.get("exchange"), Some(&"hyperliquid".to_string()));
    }
    
    #[test]
    fn test_trading_config_and_risk_config() {
        // Create risk config
        let risk_config = RiskConfig {
            max_position_size_pct: 0.1,
            max_daily_loss_pct: 0.02,
            stop_loss_pct: 0.05,
            take_profit_pct: 0.1,
            max_leverage: 3.0,
            max_positions: 5,
            max_drawdown_pct: 0.2,
            use_trailing_stop: true,
            trailing_stop_distance_pct: Some(0.02),
        };
        
        assert_eq!(risk_config.max_position_size_pct, 0.1);
        assert_eq!(risk_config.max_daily_loss_pct, 0.02);
        assert_eq!(risk_config.stop_loss_pct, 0.05);
        assert_eq!(risk_config.take_profit_pct, 0.1);
        assert_eq!(risk_config.max_leverage, 3.0);
        assert_eq!(risk_config.max_positions, 5);
        assert_eq!(risk_config.max_drawdown_pct, 0.2);
        assert_eq!(risk_config.use_trailing_stop, true);
        assert_eq!(risk_config.trailing_stop_distance_pct, Some(0.02));
        
        // Create trading config
        let mut trading_config = TradingConfig {
            initial_balance: 10000.0,
            risk_config: Some(risk_config),
            slippage_config: None,
            api_config: None,
            parameters: HashMap::new(),
        };
        
        assert_eq!(trading_config.initial_balance, 10000.0);
        assert!(trading_config.risk_config.is_some());
        assert!(trading_config.slippage_config.is_none());
        assert!(trading_config.api_config.is_none());
        
        // Add parameters
        trading_config.parameters.insert("backtest_mode".to_string(), "historical".to_string());
        assert_eq!(trading_config.parameters.get("backtest_mode"), Some(&"historical".to_string()));
    }
}