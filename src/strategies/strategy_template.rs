use std::collections::HashMap;
use chrono::{DateTime, FixedOffset, Utc};
use tracing::{info, debug, warn};

use crate::strategies::trading_strategy::{
    TradingStrategy, StrategyConfig, StrategyState, BaseTradingStrategy
};
use crate::unified_data_impl::{
    MarketData, OrderRequest, OrderResult, Signal, FundingPayment,
    SignalDirection, OrderSide, OrderType
};

/// Strategy template that can be used as a starting point for new strategies
pub struct StrategyTemplate {
    /// Base strategy implementation
    base: BaseTradingStrategy,
    
    /// Current signals by symbol
    signals: HashMap<String, Signal>,
    
    /// Custom strategy parameters
    parameter1: f64,
    parameter2: bool,
}

impl StrategyTemplate {
    /// Create a new StrategyTemplate
    pub fn new(parameter1: f64, parameter2: bool) -> Self {
        let config = StrategyConfig::new(
            "StrategyTemplate",
            "Template for creating new strategies",
            "1.0.0",
        )
        .with_number_param("parameter1", parameter1)
        .with_bool_param("parameter2", parameter2);
        
        let mut base = BaseTradingStrategy::new(
            "StrategyTemplate",
            "Template for creating new strategies",
            "1.0.0",
        );
        
        // Update the base strategy with our config
        base.update_config(config).expect("Failed to update config");
        
        Self {
            base,
            signals: HashMap::new(),
            parameter1,
            parameter2,
        }
    }
    
    /// Generate a signal based on market data
    fn generate_signal(&self, data: &MarketData) -> Signal {
        // This is where you would implement your strategy logic
        // For this template, we'll just create a simple signal based on price movement
        
        let direction = if data.price > data.mid_price() {
            SignalDirection::Buy
        } else if data.price < data.mid_price() {
            SignalDirection::Sell
        } else {
            SignalDirection::Neutral
        };
        
        let strength = if self.parameter2 {
            // Higher strength if parameter2 is true
            0.8
        } else {
            // Lower strength if parameter2 is false
            0.5
        };
        
        let mut metadata = HashMap::new();
        metadata.insert("strategy_type".to_string(), "template".to_string());
        metadata.insert("parameter1".to_string(), self.parameter1.to_string());
        metadata.insert("parameter2".to_string(), self.parameter2.to_string());
        
        Signal {
            symbol: data.symbol.clone(),
            direction,
            strength,
            timestamp: data.timestamp,
            metadata,
        }
    }
}

impl TradingStrategy for StrategyTemplate {
    fn name(&self) -> &str {
        self.base.name()
    }
    
    fn config(&self) -> &StrategyConfig {
        self.base.config()
    }
    
    fn config_mut(&mut self) -> &mut StrategyConfig {
        self.base.config_mut()
    }
    
    fn update_config(&mut self, config: StrategyConfig) -> Result<(), String> {
        // Update parameters if present in the new config
        if let Some(parameter1) = config.get_number("parameter1") {
            self.parameter1 = parameter1;
        }
        
        if let Some(parameter2) = config.get_bool("parameter2") {
            self.parameter2 = parameter2;
        }
        
        // Update the base strategy config
        self.base.update_config(config)
    }
    
    fn state(&self) -> &StrategyState {
        self.base.state()
    }
    
    fn state_mut(&mut self) -> &mut StrategyState {
        self.base.state_mut()
    }
    
    fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
        // Generate signal based on market data
        let signal = self.generate_signal(data);
        
        // Store the signal
        self.signals.insert(data.symbol.clone(), signal.clone());
        
        // Update state with signal
        self.state_mut().set_signal(&data.symbol, match signal.direction {
            SignalDirection::Buy => "long",
            SignalDirection::Sell => "short",
            _ => "neutral",
        });
        
        // Generate order requests based on the signal
        let orders = match signal.direction {
            SignalDirection::Buy => {
                // Calculate position size based on signal strength
                let position_size = signal.strength * self.parameter1;
                
                vec![OrderRequest::market(&data.symbol, OrderSide::Buy, position_size)]
            },
            SignalDirection::Sell => {
                // Calculate position size based on signal strength
                let position_size = signal.strength * self.parameter1;
                
                vec![OrderRequest::market(&data.symbol, OrderSide::Sell, position_size)]
            },
            _ => {
                // No signal, no orders
                vec![]
            }
        };
        
        // Update state timestamp
        self.state_mut().update_timestamp(data.timestamp);
        
        Ok(orders)
    }
    
    fn on_order_fill(&mut self, fill: &OrderResult) -> Result<(), String> {
        // Update position in state
        let position_size = match fill.side {
            OrderSide::Buy => fill.filled_quantity,
            OrderSide::Sell => -fill.filled_quantity,
        };
        
        // Get current position or default to 0.0
        let current_position = self.state().positions.get(&fill.symbol).copied().unwrap_or(0.0);
        
        // Update position
        self.state_mut().set_position(&fill.symbol, current_position + position_size);
        
        // Update metrics
        if let Some(price) = fill.average_price {
            self.state_mut().set_metric(&format!("{}_last_fill_price", fill.symbol), price);
        }
        
        if let Some(fees) = fill.fees {
            // Get current fees or default to 0.0
            let current_fees = self.state().metrics.get("total_fees").copied().unwrap_or(0.0);
            self.state_mut().set_metric("total_fees", current_fees + fees);
        }
        
        info!(
            "Order filled: {} {} {} at {} with size {}",
            fill.symbol,
            fill.side,
            fill.order_type,
            fill.average_price.unwrap_or(0.0),
            fill.filled_quantity
        );
        
        Ok(())
    }
    
    fn on_funding_payment(&mut self, payment: &FundingPayment) -> Result<(), String> {
        // Update funding metrics in state
        let key = format!("{}_funding_payment", payment.symbol);
        
        // Get current funding payment or default to 0.0
        let current_payment = self.state().metrics.get(&key).copied().unwrap_or(0.0);
        
        // Update with new payment
        self.state_mut().set_metric(&key, current_payment + payment.amount);
        
        // Update total funding metrics
        let total_key = if payment.amount > 0.0 {
            "total_funding_received"
        } else {
            "total_funding_paid"
        };
        
        let current_total = self.state().metrics.get(total_key).copied().unwrap_or(0.0);
        self.state_mut().set_metric(total_key, current_total + payment.amount.abs());
        
        info!(
            "Funding payment for {}: rate={}, amount={}, position={}",
            payment.symbol,
            payment.rate,
            payment.amount,
            payment.position_size
        );
        
        Ok(())
    }
    
    fn get_current_signals(&self) -> HashMap<String, Signal> {
        self.signals.clone()
    }
    
    fn initialize(&mut self) -> Result<(), String> {
        info!(
            "Initializing StrategyTemplate with parameter1={}, parameter2={}",
            self.parameter1,
            self.parameter2
        );
        Ok(())
    }
    
    fn shutdown(&mut self) -> Result<(), String> {
        info!("Shutting down StrategyTemplate");
        Ok(())
    }
}

/// Create a new strategy template with the specified parameters
pub fn create_strategy_template(parameter1: f64, parameter2: bool) -> Result<Box<dyn TradingStrategy>, String> {
    if parameter1 <= 0.0 {
        return Err("parameter1 must be positive".to_string());
    }
    
    Ok(Box::new(StrategyTemplate::new(parameter1, parameter2)))
}