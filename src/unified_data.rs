use chrono::{DateTime, FixedOffset, Utc};
use std::collections::HashMap;

pub use crate::backtest::FundingPayment;

/// Direction of an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Supported order execution types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Market,
    Limit,
}

/// Time-in-force settings for orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInForce {
    GoodTillCancel,
    ImmediateOrCancel,
    FillOrKill,
    GoodTillDate,
}

/// Basic representation of a trading position.
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub symbol: String,
    pub size: f64,
    pub entry_price: f64,
    pub current_price: f64,
    pub realized_pnl: f64,
    pub funding_pnl: f64,
    pub timestamp: DateTime<FixedOffset>,
}

impl Position {
    pub fn new(
        symbol: &str,
        size: f64,
        entry_price: f64,
        current_price: f64,
        timestamp: DateTime<FixedOffset>,
    ) -> Self {
        Self {
            symbol: symbol.to_string(),
            size,
            entry_price,
            current_price,
            realized_pnl: 0.0,
            funding_pnl: 0.0,
            timestamp,
        }
    }

    pub fn update_price(&mut self, price: f64) {
        self.current_price = price;
    }

    pub fn apply_funding_payment(&mut self, payment: f64) {
        self.funding_pnl += payment;
    }

    pub fn total_pnl(&self) -> f64 {
        self.realized_pnl + self.unrealized_pnl() + self.funding_pnl
    }

    pub fn unrealized_pnl(&self) -> f64 {
        self.size * (self.current_price - self.entry_price)
    }
}

/// Request to place an order on the exchange.
#[derive(Debug, Clone, PartialEq)]
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
}

/// Outcome of an order execution.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderResult {
    pub order_id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub quantity: f64,
    pub price: f64,
    pub timestamp: DateTime<FixedOffset>,
}

impl OrderResult {
    pub fn new(order_id: &str, symbol: &str, side: OrderSide, quantity: f64, price: f64) -> Self {
        Self {
            order_id: order_id.to_string(),
            symbol: symbol.to_string(),
            side,
            quantity,
            price,
            timestamp: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
        }
    }
}
