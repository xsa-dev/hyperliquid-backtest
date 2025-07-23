// This is a standalone example that demonstrates the unified data structures
// It doesn't depend on the existing codebase

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

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "Buy"),
            OrderSide::Sell => write!(f, "Sell"),
        }
    }
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
}

// Slippage simulation configuration for paper trading
#[derive(Debug, Clone)]
pub struct SlippageConfig {
    pub base_slippage_pct: f64,
    pub volume_impact_factor: f64,
    pub volatility_impact_factor: f64,
    pub random_slippage_max_pct: f64,
    pub simulated_latency_ms: u64,
}

// API configuration for live trading
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub api_key: String,
    pub api_secret: String,
    pub endpoint: String,
    pub use_testnet: bool,
    pub timeout_ms: u64,
}

fn main() {
    println!("Unified Data Structures Example");
    println!("===============================");
    
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Create a position
    let mut position = Position::new("BTC", 1.0, 50000.0, 51000.0, now);
    println!("Position: {} {} @ ${}", 
        if position.is_long() { "LONG" } else { "SHORT" },
        position.size,
        position.entry_price
    );
    println!("Unrealized PnL: ${:.2}", position.unrealized_pnl);
    
    // Update position price
    position.update_price(52000.0);
    println!("Updated price: ${}", position.current_price);
    println!("New unrealized PnL: ${:.2}", position.unrealized_pnl);
    
    // Apply funding payment
    position.apply_funding_payment(100.0);
    println!("After funding payment:");
    println!("Funding PnL: ${:.2}", position.funding_pnl);
    println!("Total PnL: ${:.2}", position.total_pnl());
    
    // Create an order request
    let market_order = OrderRequest::market("BTC", OrderSide::Buy, 1.0);
    println!("\nMarket Order: {} {} {}", 
        market_order.side,
        market_order.quantity,
        market_order.symbol
    );
    
    let limit_order = OrderRequest::limit("ETH", OrderSide::Sell, 2.0, 3000.0)
        .reduce_only()
        .with_time_in_force(TimeInForce::FillOrKill);
    println!("Limit Order: {} {} {} @ ${} (reduce only: {})", 
        limit_order.side,
        limit_order.quantity,
        limit_order.symbol,
        limit_order.price.unwrap(),
        limit_order.reduce_only
    );
    
    // Create market data
    let market_data = MarketData::new(
        "BTC",
        50000.0,
        49990.0,
        50010.0,
        100.0,
        now,
    );
    println!("\nMarket Data for {}:", market_data.symbol);
    println!("Price: ${}", market_data.price);
    println!("Bid/Ask: ${}/{}", market_data.bid, market_data.ask);
    println!("Spread: ${} ({:.3}%)", 
        market_data.spread(), 
        market_data.spread_percentage()
    );
    
    // Create trading configuration
    let risk_config = RiskConfig {
        max_position_size_pct: 0.1,
        max_daily_loss_pct: 0.02,
        stop_loss_pct: 0.05,
        take_profit_pct: 0.1,
        max_leverage: 3.0,
    };
    
    let trading_config = TradingConfig {
        initial_balance: 10000.0,
        risk_config: Some(risk_config),
        slippage_config: None,
        api_config: None,
        parameters: HashMap::new(),
    };
    
    println!("\nTrading Configuration:");
    println!("Initial Balance: ${}", trading_config.initial_balance);
    println!("Max Position Size: {}%", 
        trading_config.risk_config.as_ref().unwrap().max_position_size_pct * 100.0
    );
    println!("Stop Loss: {}%", 
        trading_config.risk_config.as_ref().unwrap().stop_loss_pct * 100.0
    );
    
    println!("\nExample completed successfully!");
}