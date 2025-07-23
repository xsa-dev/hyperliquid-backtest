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

/// Enhanced SMA cross strategy with funding awareness that implements the TradingStrategy trait
pub struct EnhancedSmaStrategy {
    /// Base strategy implementation
    base: BaseTradingStrategy,
    
    /// Fast period for SMA
    fast_period: usize,
    
    /// Slow period for SMA
    slow_period: usize,
    
    /// Funding-aware configuration
    funding_config: FundingAwareConfig,
    
    /// Current signals by symbol
    signals: HashMap<String, Signal>,
    
    /// Price history by symbol
    price_history: HashMap<String, Vec<f64>>,
    
    /// Last funding rates by symbol
    last_funding_rates: HashMap<String, f64>,
}

impl EnhancedSmaStrategy {
    /// Create a new EnhancedSmaStrategy
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        if fast_period >= slow_period {
            panic!("Fast period must be less than slow period");
        }
        
        let config = StrategyConfig::new(
            "EnhancedSmaStrategy",
            "Enhanced SMA cross strategy with funding awareness",
            "1.0.0",
        )
        .with_number_param("fast_period", fast_period as f64)
        .with_number_param("slow_period", slow_period as f64)
        .with_number_param("funding_weight", 0.5)
        .with_number_param("funding_threshold", 0.0001)
        .with_bool_param("use_funding_direction", true)
        .with_bool_param("use_funding_prediction", false);
        
        let mut base = BaseTradingStrategy::new(
            "EnhancedSmaStrategy",
            "Enhanced SMA cross strategy with funding awareness",
            "1.0.0",
        );
        
        // Update the base strategy with our config
        base.update_config(config).expect("Failed to update config");
        
        Self {
            base,
            fast_period,
            slow_period,
            funding_config: FundingAwareConfig::default(),
            signals: HashMap::new(),
            price_history: HashMap::new(),
            last_funding_rates: HashMap::new(),
        }
    }
    
    /// Calculate SMA for a given period
    fn calculate_sma(&self, prices: &[f64], period: usize) -> Option<f64> {
        if prices.len() < period {
            return None;
        }
        
        let sum: f64 = prices[prices.len() - period..].iter().sum();
        Some(sum / period as f64)
    }
    
    /// Process funding rate to generate a trading signal
    fn process_funding(&self, funding_rate: f64) -> TradingSignal {
        if !self.funding_config.use_funding_direction || 
           funding_rate.abs() <= self.funding_config.funding_threshold {
            return TradingSignal::new(0.0, SignalStrength::Weak);
        }
        
        let position = if funding_rate > 0.0 { 1.0 } else { -1.0 };
        let strength = if funding_rate.abs() > self.funding_config.funding_threshold * 2.0 {
            SignalStrength::Medium
        } else {
            SignalStrength::Weak
        };
        
        TradingSignal::new(position, strength)
    }
    
    /// Combine base signal with funding signal
    fn combine_signals(&self, base_signal: f64, funding_signal: &TradingSignal) -> f64 {
        if funding_signal.is_neutral() {
            return base_signal;
        }
        
        // If signals agree, strengthen the position
        if (base_signal > 0.0 && funding_signal.is_long()) || 
           (base_signal < 0.0 && funding_signal.is_short()) {
            return base_signal;
        }
        
        // If signals disagree, reduce the position based on funding strength
        let weight = match funding_signal.strength {
            SignalStrength::Strong => self.funding_config.funding_weight,
            SignalStrength::Medium => self.funding_config.funding_weight * 0.5,
            SignalStrength::Weak => self.funding_config.funding_weight * 0.2,
        };
        
        base_signal * (1.0 - weight)
    }
    
    /// Convert internal trading signal to unified Signal
    fn convert_to_signal(&self, symbol: &str, position: f64, timestamp: DateTime<FixedOffset>) -> Signal {
        let direction = if position > 0.0 {
            SignalDirection::Buy
        } else if position < 0.0 {
            SignalDirection::Sell
        } else {
            SignalDirection::Neutral
        };
        
        let strength = position.abs();
        
        let mut metadata = HashMap::new();
        metadata.insert("strategy_type".to_string(), "enhanced_sma_cross".to_string());
        metadata.insert("fast_period".to_string(), self.fast_period.to_string());
        metadata.insert("slow_period".to_string(), self.slow_period.to_string());
        
        Signal {
            symbol: symbol.to_string(),
            direction,
            strength,
            timestamp,
            metadata,
        }
    }
}

impl TradingStrategy for EnhancedSmaStrategy {
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
        // Update periods if present in the new config
        if let Some(fast_period) = config.get_number("fast_period") {
            self.fast_period = fast_period as usize;
        }
        
        if let Some(slow_period) = config.get_number("slow_period") {
            self.slow_period = slow_period as usize;
        }
        
        // Validate periods
        if self.fast_period >= self.slow_period {
            return Err("Fast period must be less than slow period".to_string());
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
        // Update price history
        let prices = self.price_history
            .entry(data.symbol.clone())
            .or_insert_with(Vec::new);
        
        prices.push(data.price);
        
        // Ensure we have enough price history
        if prices.len() < self.slow_period {
            debug!(
                "Not enough price history for {} ({}/{})",
                data.symbol,
                prices.len(),
                self.slow_period
            );
            return Ok(vec![]);
        }
        
        // Calculate SMAs
        let fast_sma = match self.calculate_sma(prices, self.fast_period) {
            Some(sma) => sma,
            None => {
                debug!("Failed to calculate fast SMA for {}", data.symbol);
                return Ok(vec![]);
            }
        };
        
        let slow_sma = match self.calculate_sma(prices, self.slow_period) {
            Some(sma) => sma,
            None => {
                debug!("Failed to calculate slow SMA for {}", data.symbol);
                return Ok(vec![]);
            }
        };
        
        // Calculate base signal from SMA cross
        let base_signal = if fast_sma > slow_sma {
            1.0 // Long signal
        } else if fast_sma < slow_sma {
            -1.0 // Short signal
        } else {
            0.0 // Neutral
        };
        
        // Get funding rate from market data
        let funding_signal = if let Some(funding_rate) = data.funding_rate {
            // Store the funding rate for future reference
            self.last_funding_rates.insert(data.symbol.clone(), funding_rate);
            
            // Process funding rate
            self.process_funding(funding_rate)
        } else if let Some(rate) = self.last_funding_rates.get(&data.symbol) {
            // Use stored funding rate
            self.process_funding(*rate)
        } else {
            // No funding rate available
            TradingSignal::new(0.0, SignalStrength::Weak)
        };
        
        // Combine signals
        let final_position = self.combine_signals(base_signal, &funding_signal);
        
        // Convert to unified Signal and store
        let signal = self.convert_to_signal(&data.symbol, final_position, data.timestamp);
        self.signals.insert(data.symbol.clone(), signal.clone());
        
        // Update state with current metrics
        self.state_mut().set_metric(&format!("{}_fast_sma", data.symbol), fast_sma);
        self.state_mut().set_metric(&format!("{}_slow_sma", data.symbol), slow_sma);
        self.state_mut().set_metric(&format!("{}_base_signal", data.symbol), base_signal);
        self.state_mut().set_metric(&format!("{}_final_signal", data.symbol), final_position);
        
        if let Some(funding_rate) = data.funding_rate {
            self.state_mut().set_metric(&format!("{}_funding_rate", data.symbol), funding_rate);
        }
        
        // Update state with signal
        self.state_mut().set_signal(&data.symbol, if final_position > 0.0 {
            "long"
        } else if final_position < 0.0 {
            "short"
        } else {
            "neutral"
        });
        
        // Generate order requests based on the final position
        let orders = if final_position != 0.0 {
            // Calculate position size (absolute value of final_position, capped at 1.0)
            let position_size = final_position.abs().min(1.0);
            
            // Create order request
            let side = if final_position > 0.0 {
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
        info!(
            "Initializing EnhancedSmaStrategy with fast_period={}, slow_period={}",
            self.fast_period,
            self.slow_period
        );
        
        // Clear price history
        self.price_history.clear();
        
        Ok(())
    }
    
    fn shutdown(&mut self) -> Result<(), String> {
        info!("Shutting down EnhancedSmaStrategy");
        Ok(())
    }
}

/// Create a new enhanced SMA cross strategy with the specified parameters
pub fn create_enhanced_sma_strategy(
    fast_period: usize,
    slow_period: usize,
    funding_config: Option<FundingAwareConfig>,
) -> Result<Box<dyn TradingStrategy>, String> {
    if fast_period >= slow_period {
        return Err("Fast period must be less than slow period".to_string());
    }
    
    let mut strategy = EnhancedSmaStrategy::new(fast_period, slow_period);
    
    // Apply custom funding config if provided
    if let Some(config) = funding_config {
        strategy.funding_config = config;
    }
    
    Ok(Box::new(strategy))
}