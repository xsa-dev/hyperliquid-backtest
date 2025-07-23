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
use crate::strategies::{FundingAwareConfig, SignalStrength, TradingSignal};

/// Funding arbitrage strategy implementation that implements the TradingStrategy trait
pub struct FundingArbitrageStrategy {
    /// Base strategy implementation
    base: BaseTradingStrategy,
    
    /// Threshold for taking positions
    threshold: f64,
    
    /// Funding-aware configuration
    funding_config: FundingAwareConfig,
    
    /// Current signals by symbol
    signals: HashMap<String, Signal>,
    
    /// Last funding rates by symbol
    last_funding_rates: HashMap<String, f64>,
}

impl FundingArbitrageStrategy {
    /// Create a new FundingArbitrageStrategy
    pub fn new(threshold: f64) -> Self {
        let config = StrategyConfig::new(
            "FundingArbitrageStrategy",
            "Strategy that exploits funding rate inefficiencies",
            "1.0.0",
        )
        .with_number_param("threshold", threshold)
        .with_number_param("funding_weight", 0.8)
        .with_bool_param("use_funding_direction", true)
        .with_bool_param("use_funding_prediction", false);
        
        let mut base = BaseTradingStrategy::new(
            "FundingArbitrageStrategy",
            "Strategy that exploits funding rate inefficiencies",
            "1.0.0",
        );
        
        // Update the base strategy with our config
        base.update_config(config).expect("Failed to update config");
        
        Self {
            base,
            threshold,
            funding_config: FundingAwareConfig::default(),
            signals: HashMap::new(),
            last_funding_rates: HashMap::new(),
        }
    }
    
    /// Process funding rate to generate a trading signal
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
    
    /// Convert internal trading signal to unified Signal
    fn convert_to_signal(&self, symbol: &str, trading_signal: &TradingSignal, timestamp: DateTime<FixedOffset>) -> Signal {
        let direction = if trading_signal.position > 0.0 {
            SignalDirection::Buy
        } else if trading_signal.position < 0.0 {
            SignalDirection::Sell
        } else {
            SignalDirection::Neutral
        };
        
        let strength = match trading_signal.strength {
            SignalStrength::Strong => 0.9,
            SignalStrength::Medium => 0.6,
            SignalStrength::Weak => 0.3,
        };
        
        let mut metadata = HashMap::new();
        metadata.insert("strategy_type".to_string(), "funding_arbitrage".to_string());
        metadata.insert("signal_source".to_string(), "funding_rate".to_string());
        
        Signal {
            symbol: symbol.to_string(),
            direction,
            strength,
            timestamp,
            metadata,
        }
    }
}

impl TradingStrategy for FundingArbitrageStrategy {
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
        // Update the threshold if present in the new config
        if let Some(threshold) = config.get_number("threshold") {
            self.threshold = threshold;
        }
        
        // Update funding config if parameters are present
        let mut funding_config = self.funding_config.clone();
        
        if let Some(funding_weight) = config.get_number("funding_weight") {
            funding_config.funding_weight = funding_weight;
        }
        
        if let Some(funding_threshold) = config.get_number("funding_threshold") {
            funding_config.funding_threshold = funding_threshold;
        }
        
        if let Some(use_funding_direction) = config.get_bool("use_funding_direction") {
            funding_config.use_funding_direction = use_funding_direction;
        }
        
        if let Some(use_funding_prediction) = config.get_bool("use_funding_prediction") {
            funding_config.use_funding_prediction = use_funding_prediction;
        }
        
        self.funding_config = funding_config;
        
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
        // Get funding rate from market data
        let funding_rate = match data.funding_rate {
            Some(rate) => rate,
            None => {
                // If no funding rate is available, check if we have a stored one
                match self.last_funding_rates.get(&data.symbol) {
                    Some(rate) => *rate,
                    None => {
                        debug!("No funding rate available for {}, skipping signal generation", data.symbol);
                        return Ok(vec![]);
                    }
                }
            }
        };
        
        // Store the funding rate for future reference
        self.last_funding_rates.insert(data.symbol.clone(), funding_rate);
        
        // Process funding rate to generate a trading signal
        let trading_signal = self.process_funding(funding_rate);
        
        // Convert to unified Signal and store
        let signal = self.convert_to_signal(&data.symbol, &trading_signal, data.timestamp);
        self.signals.insert(data.symbol.clone(), signal);
        
        // Update state with current funding rate and signal
        self.state_mut().set_metric(&format!("{}_funding_rate", data.symbol), funding_rate);
        self.state_mut().set_signal(&data.symbol, if trading_signal.position > 0.0 {
            "long"
        } else if trading_signal.position < 0.0 {
            "short"
        } else {
            "neutral"
        });
        
        // Generate order requests based on the trading signal
        let orders = if !trading_signal.is_neutral() {
            // Calculate position size based on signal strength
            let position_size = match trading_signal.strength {
                SignalStrength::Strong => 1.0,
                SignalStrength::Medium => 0.7,
                SignalStrength::Weak => 0.3,
            };
            
            // Create order request
            let side = if trading_signal.is_long() {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            };
            
            vec![OrderRequest::market(&data.symbol, side, position_size)]
        } else {
            // No signal, no orders
            vec![]
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
        info!("Initializing FundingArbitrageStrategy with threshold {}", self.threshold);
        Ok(())
    }
    
    fn shutdown(&mut self) -> Result<(), String> {
        info!("Shutting down FundingArbitrageStrategy");
        Ok(())
    }
}

/// Create a new funding arbitrage strategy with the specified threshold
pub fn create_funding_arbitrage_strategy(threshold: f64) -> Result<Box<dyn TradingStrategy>, String> {
    if threshold <= 0.0 {
        return Err("Threshold must be positive".to_string());
    }
    
    Ok(Box::new(FundingArbitrageStrategy::new(threshold)))
}