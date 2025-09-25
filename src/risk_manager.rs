use std::collections::HashMap;

use chrono::{DateTime, FixedOffset, Utc};
use thiserror::Error;

use crate::unified_data::{OrderRequest, OrderSide, OrderType, Position};

/// Configuration values used by the [`RiskManager`].
#[derive(Debug, Clone)]
pub struct RiskConfig {
    pub max_position_size_pct: f64,
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_size_pct: 0.1,
            stop_loss_pct: 0.05,
            take_profit_pct: 0.1,
        }
    }
}

/// Errors that can be returned by [`RiskManager`].
#[derive(Debug, Error, Clone)]
pub enum RiskError {
    /// Returned when an order would exceed the configured position size.
    #[error("position size exceeds configured limit: {message}")]
    PositionSizeExceeded { message: String },
    /// Returned when trading is halted by the emergency stop flag.
    #[error("trading is halted by the emergency stop toggle")]
    TradingHalted,
}

/// Convenience result type for risk management operations.
pub type Result<T> = std::result::Result<T, RiskError>;

/// Representation of a stop-loss or take-profit order managed by [`RiskManager`].
#[derive(Debug, Clone)]
pub struct RiskOrder {
    /// Identifier of the originating order.
    pub parent_order_id: String,
    /// Asset symbol.
    pub symbol: String,
    /// Order side used to flatten the position when triggered.
    pub side: OrderSide,
    /// Order type used when submitting the risk order.
    pub order_type: OrderType,
    /// Quantity to trade when the order triggers.
    pub quantity: f64,
    /// Trigger price for the order.
    pub trigger_price: f64,
    /// Whether the order acts as a stop-loss.
    pub is_stop_loss: bool,
    /// Whether the order acts as a take-profit.
    pub is_take_profit: bool,
    /// Timestamp when the risk order was created.
    pub created_at: DateTime<FixedOffset>,
}

impl RiskOrder {
    fn new(
        parent_order_id: &str,
        symbol: &str,
        side: OrderSide,
        quantity: f64,
        trigger_price: f64,
        is_stop_loss: bool,
        is_take_profit: bool,
    ) -> Self {
        Self {
            parent_order_id: parent_order_id.to_string(),
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Market,
            quantity,
            trigger_price,
            is_stop_loss,
            is_take_profit,
            created_at: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
        }
    }
}

/// Minimal risk management component used by the higher level trading engines.
#[derive(Debug, Clone)]
pub struct RiskManager {
    config: RiskConfig,
    portfolio_value: f64,
    stop_losses: Vec<RiskOrder>,
    take_profits: Vec<RiskOrder>,
    emergency_stop: bool,
}

impl RiskManager {
    /// Create a new [`RiskManager`] with the provided configuration.
    pub fn new(config: RiskConfig, portfolio_value: f64) -> Self {
        Self {
            config,
            portfolio_value,
            stop_losses: Vec::new(),
            take_profits: Vec::new(),
            emergency_stop: false,
        }
    }

    /// Access the underlying risk configuration.
    pub fn config(&self) -> &RiskConfig {
        &self.config
    }

    /// Update the tracked portfolio value. The current implementation simply records
    /// the latest value so that position size checks have an up-to-date notion of the
    /// account size.
    pub fn update_portfolio_value(
        &mut self,
        new_value: f64,
        _realized_pnl_delta: f64,
    ) -> Result<()> {
        self.portfolio_value = new_value.max(0.0);
        Ok(())
    }

    /// Validate an order against simple position size limits and the emergency stop flag.
    pub fn validate_order(
        &self,
        order: &OrderRequest,
        _positions: &HashMap<String, Position>,
    ) -> Result<()> {
        if self.emergency_stop {
            return Err(RiskError::TradingHalted);
        }

        if let Some(price) = order.price {
            let notional = price * order.quantity.abs();
            let max_notional = self.config.max_position_size_pct * self.portfolio_value;
            if max_notional > 0.0 && notional > max_notional {
                return Err(RiskError::PositionSizeExceeded {
                    message: format!(
                        "order notional {:.2} exceeds {:.2} ({:.2}% of portfolio)",
                        notional,
                        max_notional,
                        self.config.max_position_size_pct * 100.0,
                    ),
                });
            }
        }

        Ok(())
    }

    /// Produce a stop-loss order for the supplied position.
    pub fn generate_stop_loss(&self, position: &Position, order_id: &str) -> Option<RiskOrder> {
        if position.size == 0.0 || self.config.stop_loss_pct <= 0.0 {
            return None;
        }

        let trigger_price = if position.size > 0.0 {
            position.entry_price * (1.0 - self.config.stop_loss_pct)
        } else {
            position.entry_price * (1.0 + self.config.stop_loss_pct)
        };

        let side = if position.size > 0.0 {
            OrderSide::Sell
        } else {
            OrderSide::Buy
        };

        Some(RiskOrder::new(
            order_id,
            &position.symbol,
            side,
            position.size.abs(),
            trigger_price,
            true,
            false,
        ))
    }

    /// Produce a take-profit order for the supplied position.
    pub fn generate_take_profit(&self, position: &Position, order_id: &str) -> Option<RiskOrder> {
        if position.size == 0.0 || self.config.take_profit_pct <= 0.0 {
            return None;
        }

        let trigger_price = if position.size > 0.0 {
            position.entry_price * (1.0 + self.config.take_profit_pct)
        } else {
            position.entry_price * (1.0 - self.config.take_profit_pct)
        };

        let side = if position.size > 0.0 {
            OrderSide::Sell
        } else {
            OrderSide::Buy
        };

        Some(RiskOrder::new(
            order_id,
            &position.symbol,
            side,
            position.size.abs(),
            trigger_price,
            false,
            true,
        ))
    }

    /// Store a generated stop-loss order.
    pub fn register_stop_loss(&mut self, order: RiskOrder) {
        self.stop_losses.push(order);
    }

    /// Store a generated take-profit order.
    pub fn register_take_profit(&mut self, order: RiskOrder) {
        self.take_profits.push(order);
    }

    /// Inspect tracked risk orders against the latest market prices.
    pub fn check_risk_orders(&mut self, current_prices: &HashMap<String, f64>) -> Vec<RiskOrder> {
        fn should_trigger(order: &RiskOrder, price: f64) -> bool {
            if order.is_stop_loss {
                match order.side {
                    OrderSide::Sell => price <= order.trigger_price,
                    OrderSide::Buy => price >= order.trigger_price,
                }
            } else if order.is_take_profit {
                match order.side {
                    OrderSide::Sell => price >= order.trigger_price,
                    OrderSide::Buy => price <= order.trigger_price,
                }
            } else {
                false
            }
        }

        let mut triggered = Vec::new();

        self.stop_losses.retain(|order| {
            if let Some(price) = current_prices.get(&order.symbol) {
                if should_trigger(order, *price) {
                    triggered.push(order.clone());
                    return false;
                }
            }
            true
        });

        self.take_profits.retain(|order| {
            if let Some(price) = current_prices.get(&order.symbol) {
                if should_trigger(order, *price) {
                    triggered.push(order.clone());
                    return false;
                }
            }
            true
        });

        triggered
    }

    /// Manually trigger the emergency stop.
    pub fn activate_emergency_stop(&mut self) {
        self.emergency_stop = true;
    }

    /// Clear the emergency stop condition.
    pub fn deactivate_emergency_stop(&mut self) {
        self.emergency_stop = false;
    }

    /// Check whether trading should be halted.
    pub fn should_stop_trading(&self) -> bool {
        self.emergency_stop
    }
}
