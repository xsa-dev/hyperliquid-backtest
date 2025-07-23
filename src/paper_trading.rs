use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use rand::distributions::Distribution;
use rand_distr::Normal;
use rand::thread_rng;
use chrono::{DateTime, FixedOffset, Utc};
use log::{info, warn, error};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::trading_mode::SlippageConfig;
use crate::unified_data::{
    Position, OrderRequest, OrderResult, MarketData, 
    OrderSide, OrderType, OrderStatus,
    TradingStrategy
};
use crate::real_time_data_stream::RealTimeDataStream;

/// Error types specific to paper trading operations
#[derive(Debug, Error)]
pub enum PaperTradingError {
    /// Error when market data is not available
    #[error("Market data not available for {0}")]
    MarketDataNotAvailable(String),
    
    /// Error when order execution fails
    #[error("Order execution failed: {0}")]
    OrderExecutionFailed(String),
    
    /// Error when position is not found
    #[error("Position not found for {0}")]
    PositionNotFound(String),
    
    /// Error when insufficient balance
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance {
        required: f64,
        available: f64,
    },
    
    /// Error when real-time data stream fails
    #[error("Real-time data stream error: {0}")]
    RealTimeDataError(String),
    
    /// Error when strategy execution fails
    #[error("Strategy execution error: {0}")]
    StrategyError(String),
}

/// Represents a simulated order in the paper trading system
#[derive(Debug, Clone)]
pub struct SimulatedOrder {
    /// Order ID
    pub order_id: String,
    
    /// Original order request
    pub request: OrderRequest,
    
    /// Order result
    pub result: OrderResult,
    
    /// Creation timestamp
    pub created_at: DateTime<FixedOffset>,
    
    /// Last update timestamp
    pub updated_at: DateTime<FixedOffset>,
    
    /// Execution delay in milliseconds
    pub execution_delay_ms: u64,
    
    /// Slippage applied (percentage)
    pub slippage_pct: f64,
}

/// Paper trading engine for simulating trading with real-time data
pub struct PaperTradingEngine {
    /// Simulated account balance
    simulated_balance: f64,
    
    /// Simulated positions
    simulated_positions: HashMap<String, Position>,
    
    /// Order history
    order_history: Vec<SimulatedOrder>,
    
    /// Active orders
    active_orders: HashMap<String, SimulatedOrder>,
    
    /// Real-time data stream
    real_time_data: Option<Arc<Mutex<RealTimeDataStream>>>,
    
    /// Latest market data
    market_data_cache: HashMap<String, MarketData>,
    
    /// Slippage model configuration
    slippage_config: SlippageConfig,
    
    /// Trading fees (maker and taker)
    maker_fee: f64,
    taker_fee: f64,
    
    /// Performance metrics
    metrics: PaperTradingMetrics,
    
    /// Trade log
    trade_log: Vec<TradeLogEntry>,
    
    /// Is simulation running
    is_running: bool,
    
    /// Last update timestamp
    last_update: DateTime<FixedOffset>,
}

/// Performance metrics for paper trading
#[derive(Debug, Clone)]
pub struct PaperTradingMetrics {
    /// Initial balance
    pub initial_balance: f64,
    
    /// Current balance
    pub current_balance: f64,
    
    /// Realized profit and loss
    pub realized_pnl: f64,
    
    /// Unrealized profit and loss
    pub unrealized_pnl: f64,
    
    /// Funding profit and loss
    pub funding_pnl: f64,
    
    /// Total fees paid
    pub total_fees: f64,
    
    /// Number of trades
    pub trade_count: usize,
    
    /// Number of winning trades
    pub winning_trades: usize,
    
    /// Number of losing trades
    pub losing_trades: usize,
    
    /// Maximum drawdown
    pub max_drawdown: f64,
    
    /// Maximum drawdown percentage
    pub max_drawdown_pct: f64,
    
    /// Peak balance
    pub peak_balance: f64,
    
    /// Start time
    pub start_time: DateTime<FixedOffset>,
    
    /// Last update time
    pub last_update: DateTime<FixedOffset>,
}

/// Trade log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeLogEntry {
    /// Entry ID
    pub id: String,
    
    /// Symbol
    pub symbol: String,
    
    /// Side (buy/sell)
    pub side: OrderSide,
    
    /// Quantity
    pub quantity: f64,
    
    /// Price
    pub price: f64,
    
    /// Timestamp
    pub timestamp: DateTime<FixedOffset>,
    
    /// Fees paid
    pub fees: f64,
    
    /// Order type
    pub order_type: OrderType,
    
    /// Related order ID
    pub order_id: String,
    
    /// Profit and loss for this trade
    pub pnl: Option<f64>,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl PaperTradingEngine {
    /// Create a new paper trading engine with the specified initial balance and slippage configuration
    pub fn new(initial_balance: f64, slippage_config: SlippageConfig) -> Self {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        
        let metrics = PaperTradingMetrics {
            initial_balance,
            current_balance: initial_balance,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
            funding_pnl: 0.0,
            total_fees: 0.0,
            trade_count: 0,
            winning_trades: 0,
            losing_trades: 0,
            max_drawdown: 0.0,
            max_drawdown_pct: 0.0,
            peak_balance: initial_balance,
            start_time: now,
            last_update: now,
        };
        
        Self {
            simulated_balance: initial_balance,
            simulated_positions: HashMap::new(),
            order_history: Vec::new(),
            active_orders: HashMap::new(),
            real_time_data: None,
            market_data_cache: HashMap::new(),
            slippage_config,
            maker_fee: 0.0002, // 0.02% maker fee
            taker_fee: 0.0005, // 0.05% taker fee
            metrics,
            trade_log: Vec::new(),
            is_running: false,
            last_update: now,
        }
    }
    
    /// Set the real-time data stream
    pub fn set_real_time_data(&mut self, data_stream: Arc<Mutex<RealTimeDataStream>>) {
        self.real_time_data = Some(data_stream);
    }
    
    /// Set the fee rates
    pub fn set_fees(&mut self, maker_fee: f64, taker_fee: f64) {
        self.maker_fee = maker_fee;
        self.taker_fee = taker_fee;
    }
    
    /// Get the current simulated balance
    pub fn get_balance(&self) -> f64 {
        self.simulated_balance
    }
    
    /// Get the current positions
    pub fn get_positions(&self) -> &HashMap<String, Position> {
        &self.simulated_positions
    }
    
    /// Get the order history
    pub fn get_order_history(&self) -> &Vec<SimulatedOrder> {
        &self.order_history
    }
    
    /// Get the active orders
    pub fn get_active_orders(&self) -> &HashMap<String, SimulatedOrder> {
        &self.active_orders
    }
    
    /// Get the trade log
    pub fn get_trade_log(&self) -> &Vec<TradeLogEntry> {
        &self.trade_log
    }
    
    /// Get the performance metrics
    pub fn get_metrics(&self) -> &PaperTradingMetrics {
        &self.metrics
    }
    
    /// Get the portfolio value (balance + position values)
    pub fn get_portfolio_value(&self) -> f64 {
        let position_value = self.simulated_positions.values()
            .map(|p| p.notional_value())
            .sum::<f64>();
        
        self.simulated_balance + position_value
    }
    
    /// Update market data
    pub fn update_market_data(&mut self, data: MarketData) -> Result<(), PaperTradingError> {
        // Update the market data cache
        self.market_data_cache.insert(data.symbol.clone(), data.clone());
        
        // Update position prices if we have a position in this symbol
        if let Some(position) = self.simulated_positions.get_mut(&data.symbol) {
            position.update_price(data.price);
        }
        
        // Process any active orders that might be affected by this price update
        self.process_active_orders(&data)?;
        
        // Update metrics
        self.update_metrics();
        
        Ok(())
    }
    
    /// Execute an order
    pub async fn execute_order(&mut self, order: OrderRequest) -> Result<OrderResult, PaperTradingError> {
        // Validate the order
        if let Err(err) = order.validate() {
            return Err(PaperTradingError::OrderExecutionFailed(err));
        }
        
        // Get the latest market data for this symbol
        let market_data = self.get_market_data(&order.symbol)?;
        
        // Generate a unique order ID
        let order_id = Uuid::new_v4().to_string();
        
        // Create the initial order result
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        let mut order_result = OrderResult::new(
            &order_id,
            &order.symbol,
            order.side,
            order.order_type,
            order.quantity,
            now,
        );
        order_result.status = OrderStatus::Submitted;
        
        // For market orders, execute immediately with simulated slippage
        if order.order_type == OrderType::Market {
            // Calculate execution price with slippage
            let execution_price = self.calculate_execution_price(&order, &market_data);
            
            // Calculate fees
            let fee_rate = self.taker_fee; // Market orders are taker orders
            let fee_amount = order.quantity * execution_price * fee_rate;
            
            // Update the order result
            order_result.status = OrderStatus::Filled;
            order_result.filled_quantity = order.quantity;
            order_result.average_price = Some(execution_price);
            order_result.fees = Some(fee_amount);
            
            // Update positions and balance
            self.update_position_and_balance(&order, execution_price, fee_amount)?;
            
            // Add to order history
            let simulated_order = SimulatedOrder {
                order_id: order_id.clone(),
                request: order.clone(),
                result: order_result.clone(),
                created_at: now,
                updated_at: now,
                execution_delay_ms: self.slippage_config.simulated_latency_ms,
                slippage_pct: (execution_price - market_data.price) / market_data.price * 100.0,
            };
            self.order_history.push(simulated_order);
            
            // Add to trade log
            self.add_trade_log_entry(&order, &order_result);
            
            // Update metrics
            self.update_metrics();
            
            return Ok(order_result);
        } else {
            // For limit orders, add to active orders
            let simulated_order = SimulatedOrder {
                order_id: order_id.clone(),
                request: order.clone(),
                result: order_result.clone(),
                created_at: now,
                updated_at: now,
                execution_delay_ms: self.slippage_config.simulated_latency_ms,
                slippage_pct: 0.0, // Will be calculated on execution
            };
            
            self.active_orders.insert(order_id.clone(), simulated_order);
            
            return Ok(order_result);
        }
    }
    
    /// Cancel an order
    pub fn cancel_order(&mut self, order_id: &str) -> Result<OrderResult, PaperTradingError> {
        // Check if the order exists
        if let Some(mut order) = self.active_orders.remove(order_id) {
            // Update the order result
            let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
            order.result.status = OrderStatus::Cancelled;
            order.updated_at = now;
            
            // Add to order history
            self.order_history.push(order.clone());
            
            return Ok(order.result);
        } else {
            return Err(PaperTradingError::OrderExecutionFailed(
                format!("Order not found: {}", order_id)
            ));
        }
    }
    
    /// Start the paper trading simulation with the given strategy
    pub async fn start_simulation(&mut self, strategy: Box<dyn TradingStrategy>) -> Result<(), PaperTradingError> {
        if self.real_time_data.is_none() {
            return Err(PaperTradingError::RealTimeDataError(
                "Real-time data stream not set".to_string()
            ));
        }
        
        self.is_running = true;
        info!("Starting paper trading simulation with strategy: {}", strategy.name());
        
        // Main simulation loop
        while self.is_running {
            // Process market data updates
            self.process_market_data_updates(strategy.as_ref()).await?;
            
            // Simulate some delay to avoid CPU spinning
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        info!("Paper trading simulation stopped");
        Ok(())
    }
    
    /// Stop the paper trading simulation
    pub fn stop_simulation(&mut self) {
        self.is_running = false;
        info!("Stopping paper trading simulation");
    }
    
    /// Apply a funding payment
    pub fn apply_funding_payment(&mut self, symbol: &str, payment: f64) -> Result<(), PaperTradingError> {
        if let Some(position) = self.simulated_positions.get_mut(symbol) {
            position.apply_funding_payment(payment);
            self.metrics.funding_pnl += payment;
            Ok(())
        } else {
            Err(PaperTradingError::PositionNotFound(symbol.to_string()))
        }
    }
    
    /// Get the current market data for a symbol
    fn get_market_data(&self, symbol: &str) -> Result<MarketData, PaperTradingError> {
        if let Some(data) = self.market_data_cache.get(symbol) {
            Ok(data.clone())
        } else {
            Err(PaperTradingError::MarketDataNotAvailable(symbol.to_string()))
        }
    }
    
    /// Calculate the execution price with slippage
    fn calculate_execution_price(&self, order: &OrderRequest, market_data: &MarketData) -> f64 {
        let base_price = match order.side {
            OrderSide::Buy => market_data.ask,  // Buy at ask price
            OrderSide::Sell => market_data.bid, // Sell at bid price
        };
        
        // Calculate slippage based on configuration
        let mut slippage_pct = self.slippage_config.base_slippage_pct;
        
        // Add volume-based slippage
        let volume_impact = order.quantity / market_data.volume * self.slippage_config.volume_impact_factor;
        slippage_pct += volume_impact;
        
        // Add random slippage component
        let mut rng = thread_rng();
        let normal = Normal::new(0.0, self.slippage_config.random_slippage_max_pct / 2.0).unwrap();
        let random_slippage = normal.sample(&mut rng);
        slippage_pct += random_slippage;
        
        // Cap slippage at maximum
        slippage_pct = slippage_pct.min(0.01); // Cap at 1% max slippage
        
        // Apply slippage to price
        let slippage_factor = match order.side {
            OrderSide::Buy => 1.0 + slippage_pct,  // Higher price for buys
            OrderSide::Sell => 1.0 - slippage_pct, // Lower price for sells
        };
        
        base_price * slippage_factor
    }
    
    /// Update position and balance after an order execution
    fn update_position_and_balance(&mut self, order: &OrderRequest, execution_price: f64, fee_amount: f64) -> Result<(), PaperTradingError> {
        let symbol = &order.symbol;
        let quantity = order.quantity;
        let side = order.side;
        
        // Calculate the order cost
        let order_cost = quantity * execution_price;
        
        // Check if we have enough balance for a buy order
        if side == OrderSide::Buy && order_cost + fee_amount > self.simulated_balance {
            return Err(PaperTradingError::InsufficientBalance {
                required: order_cost + fee_amount,
                available: self.simulated_balance,
            });
        }
        
        // Update the position
        let position = self.simulated_positions.entry(symbol.clone())
            .or_insert_with(|| {
                let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
                Position::new(symbol, 0.0, 0.0, execution_price, now)
            });
        
        // Calculate the position change
        let position_change = match side {
            OrderSide::Buy => quantity,
            OrderSide::Sell => -quantity,
        };
        
        // If this is a closing trade, calculate PnL
        let mut realized_pnl = 0.0;
        if (position.size > 0.0 && position_change < 0.0) || (position.size < 0.0 && position_change > 0.0) {
            // Closing trade (partial or full)
            let closing_size = position_change.abs().min(position.size.abs());
            realized_pnl = closing_size * (execution_price - position.entry_price) * position.size.signum();
            
            // Update metrics
            self.metrics.realized_pnl += realized_pnl;
            self.metrics.trade_count += 1;
            if realized_pnl > 0.0 {
                self.metrics.winning_trades += 1;
            } else if realized_pnl < 0.0 {
                self.metrics.losing_trades += 1;
            }
        }
        
        // Update position size and entry price
        if position.size + position_change == 0.0 {
            // Position closed completely
            position.size = 0.0;
            position.entry_price = 0.0;
        } else if position.size * (position.size + position_change) < 0.0 {
            // Position flipped from long to short or vice versa
            position.size = position_change;
            position.entry_price = execution_price;
        } else if position.size == 0.0 {
            // New position
            position.size = position_change;
            position.entry_price = execution_price;
        } else {
            // Position increased or partially decreased
            let old_notional = position.size.abs() * position.entry_price;
            let new_notional = position_change.abs() * execution_price;
            let total_size = position.size + position_change;
            
            if total_size != 0.0 {
                position.entry_price = (old_notional + new_notional) / total_size.abs();
            }
            position.size = total_size;
        }
        
        // Update current price
        position.current_price = execution_price;
        
        // Update balance
        match side {
            OrderSide::Buy => {
                self.simulated_balance -= order_cost + fee_amount;
            },
            OrderSide::Sell => {
                self.simulated_balance += order_cost - fee_amount;
                self.simulated_balance += realized_pnl;
            },
        }
        
        // Update metrics
        self.metrics.total_fees += fee_amount;
        self.update_metrics();
        
        Ok(())
    }
    
    /// Process active orders based on new market data
    fn process_active_orders(&mut self, market_data: &MarketData) -> Result<(), PaperTradingError> {
        let symbol = &market_data.symbol;
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        
        // Collect orders that need to be processed
        let orders_to_process: Vec<String> = self.active_orders.iter()
            .filter(|(_, order)| order.request.symbol == *symbol)
            .map(|(id, _)| id.clone())
            .collect();
        
        for order_id in orders_to_process {
            if let Some(mut order) = self.active_orders.remove(&order_id) {
                let request = &order.request;
                
                // Check if the order should be executed based on price
                let should_execute = match (request.order_type, request.side) {
                    (OrderType::Limit, OrderSide::Buy) => {
                        // Buy limit: execute if ask price <= limit price
                        if let Some(limit_price) = request.price {
                            market_data.ask <= limit_price
                        } else {
                            false
                        }
                    },
                    (OrderType::Limit, OrderSide::Sell) => {
                        // Sell limit: execute if bid price >= limit price
                        if let Some(limit_price) = request.price {
                            market_data.bid >= limit_price
                        } else {
                            false
                        }
                    },
                    (OrderType::StopMarket, OrderSide::Buy) => {
                        // Buy stop: execute if price >= stop price
                        if let Some(stop_price) = request.stop_price {
                            market_data.price >= stop_price
                        } else {
                            false
                        }
                    },
                    (OrderType::StopMarket, OrderSide::Sell) => {
                        // Sell stop: execute if price <= stop price
                        if let Some(stop_price) = request.stop_price {
                            market_data.price <= stop_price
                        } else {
                            false
                        }
                    },
                    _ => false, // Other order types not implemented yet
                };
                
                if should_execute {
                    // Calculate execution price
                    let execution_price = match request.order_type {
                        OrderType::Limit => request.price.unwrap_or(market_data.price),
                        _ => self.calculate_execution_price(request, market_data),
                    };
                    
                    // Calculate fees
                    let fee_rate = match request.order_type {
                        OrderType::Limit => self.maker_fee, // Limit orders are maker orders
                        _ => self.taker_fee,                // Other orders are taker orders
                    };
                    let fee_amount = request.quantity * execution_price * fee_rate;
                    
                    // Update the order result
                    order.result.status = OrderStatus::Filled;
                    order.result.filled_quantity = request.quantity;
                    order.result.average_price = Some(execution_price);
                    order.result.fees = Some(fee_amount);
                    order.updated_at = now;
                    
                    // Update positions and balance
                    if let Err(err) = self.update_position_and_balance(request, execution_price, fee_amount) {
                        // If there's an error (e.g., insufficient balance), reject the order
                        order.result.status = OrderStatus::Rejected;
                        order.result.error = Some(err.to_string());
                    }
                    
                    // Add to order history
                    self.order_history.push(order.clone());
                    
                    // Add to trade log if executed
                    if order.result.status == OrderStatus::Filled {
                        self.add_trade_log_entry(request, &order.result);
                    }
                } else {
                    // Order not executed yet, put it back in active orders
                    self.active_orders.insert(order_id, order);
                }
            }
        }
        
        Ok(())
    }
    
    /// Process market data updates and execute strategy
    async fn process_market_data_updates(&mut self, strategy: &dyn TradingStrategy) -> Result<(), PaperTradingError> {
        // Get the latest market data from the real-time stream
        if let Some(data_stream) = &self.real_time_data {
            if let Ok(stream) = data_stream.lock() {
                // This is a placeholder since we don't have the actual RealTimeDataStream implementation
                // In a real implementation, we would get the latest data from the stream
                // For now, we'll just use the cached data
            }
        }
        
        // Collect market data to avoid borrowing issues
        let market_data_vec: Vec<_> = self.market_data_cache.values().cloned().collect();
        
        // Process each market data update
        for market_data in market_data_vec {
            // Execute strategy on market data (placeholder - would need mutable strategy)
            // match strategy.on_market_data(&market_data) {
            match Ok(vec![]) as Result<Vec<OrderRequest>, String> {
                Ok(order_requests) => {
                    // Execute any orders generated by the strategy
                    for order_request in order_requests {
                        match self.execute_order(order_request).await {
                            Ok(_) => {},
                            Err(err) => {
                                warn!("Failed to execute order: {}", err);
                            }
                        }
                    }
                },
                Err(err) => {
                    return Err(PaperTradingError::StrategyError(err));
                }
            }
        }
        
        Ok(())
    }
    
    /// Add a trade log entry
    fn add_trade_log_entry(&mut self, order: &OrderRequest, result: &OrderResult) {
        if result.status != OrderStatus::Filled || result.average_price.is_none() {
            return;
        }
        
        let entry = TradeLogEntry {
            id: Uuid::new_v4().to_string(),
            symbol: order.symbol.clone(),
            side: order.side,
            quantity: result.filled_quantity,
            price: result.average_price.unwrap(),
            timestamp: result.timestamp,
            fees: result.fees.unwrap_or(0.0),
            order_type: order.order_type,
            order_id: result.order_id.clone(),
            pnl: None, // Will be calculated later for closing trades
            metadata: HashMap::new(),
        };
        
        self.trade_log.push(entry);
    }
    
    /// Update performance metrics
    fn update_metrics(&mut self) {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        
        // Calculate unrealized PnL
        let unrealized_pnl = self.simulated_positions.values()
            .map(|p| p.unrealized_pnl)
            .sum::<f64>();
        
        // Calculate funding PnL
        let funding_pnl = self.simulated_positions.values()
            .map(|p| p.funding_pnl)
            .sum::<f64>();
        
        // Update metrics
        self.metrics.current_balance = self.simulated_balance;
        self.metrics.unrealized_pnl = unrealized_pnl;
        self.metrics.funding_pnl = funding_pnl;
        self.metrics.last_update = now;
        
        // Update peak balance and drawdown
        let total_equity = self.simulated_balance + unrealized_pnl + funding_pnl;
        if total_equity > self.metrics.peak_balance {
            self.metrics.peak_balance = total_equity;
        } else {
            let drawdown = self.metrics.peak_balance - total_equity;
            let drawdown_pct = if self.metrics.peak_balance > 0.0 {
                drawdown / self.metrics.peak_balance * 100.0
            } else {
                0.0
            };
            
            if drawdown > self.metrics.max_drawdown {
                self.metrics.max_drawdown = drawdown;
                self.metrics.max_drawdown_pct = drawdown_pct;
            }
        }
    }
    
    /// Generate a performance report
    pub fn generate_report(&self) -> PaperTradingReport {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        let duration = now.signed_duration_since(self.metrics.start_time);
        let duration_days = duration.num_milliseconds() as f64 / (1000.0 * 60.0 * 60.0 * 24.0);
        
        let total_equity = self.simulated_balance + self.metrics.unrealized_pnl + self.metrics.funding_pnl;
        let total_return = total_equity - self.metrics.initial_balance;
        let total_return_pct = if self.metrics.initial_balance > 0.0 {
            total_return / self.metrics.initial_balance * 100.0
        } else {
            0.0
        };
        
        let annualized_return = if duration_days > 0.0 {
            (total_return_pct / 100.0 + 1.0).powf(365.0 / duration_days) - 1.0
        } else {
            0.0
        } * 100.0;
        
        let win_rate = if self.metrics.trade_count > 0 {
            self.metrics.winning_trades as f64 / self.metrics.trade_count as f64 * 100.0
        } else {
            0.0
        };
        
        PaperTradingReport {
            initial_balance: self.metrics.initial_balance,
            current_balance: self.simulated_balance,
            unrealized_pnl: self.metrics.unrealized_pnl,
            realized_pnl: self.metrics.realized_pnl,
            funding_pnl: self.metrics.funding_pnl,
            total_pnl: self.metrics.realized_pnl + self.metrics.unrealized_pnl + self.metrics.funding_pnl,
            total_fees: self.metrics.total_fees,
            total_equity,
            total_return,
            total_return_pct,
            annualized_return,
            trade_count: self.metrics.trade_count,
            winning_trades: self.metrics.winning_trades,
            losing_trades: self.metrics.losing_trades,
            win_rate,
            max_drawdown: self.metrics.max_drawdown,
            max_drawdown_pct: self.metrics.max_drawdown_pct,
            start_time: self.metrics.start_time,
            end_time: now,
            duration_days,
        }
    }
}

/// Paper trading performance report
#[derive(Debug, Clone)]
pub struct PaperTradingReport {
    /// Initial balance
    pub initial_balance: f64,
    
    /// Current balance
    pub current_balance: f64,
    
    /// Unrealized profit and loss
    pub unrealized_pnl: f64,
    
    /// Realized profit and loss
    pub realized_pnl: f64,
    
    /// Funding profit and loss
    pub funding_pnl: f64,
    
    /// Total profit and loss
    pub total_pnl: f64,
    
    /// Total fees paid
    pub total_fees: f64,
    
    /// Total equity (balance + unrealized PnL)
    pub total_equity: f64,
    
    /// Total return (absolute)
    pub total_return: f64,
    
    /// Total return (percentage)
    pub total_return_pct: f64,
    
    /// Annualized return (percentage)
    pub annualized_return: f64,
    
    /// Number of trades
    pub trade_count: usize,
    
    /// Number of winning trades
    pub winning_trades: usize,
    
    /// Number of losing trades
    pub losing_trades: usize,
    
    /// Win rate (percentage)
    pub win_rate: f64,
    
    /// Maximum drawdown
    pub max_drawdown: f64,
    
    /// Maximum drawdown percentage
    pub max_drawdown_pct: f64,
    
    /// Start time
    pub start_time: DateTime<FixedOffset>,
    
    /// End time
    pub end_time: DateTime<FixedOffset>,
    
    /// Duration in days
    pub duration_days: f64,
}

impl std::fmt::Display for PaperTradingReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Paper Trading Performance Report ===")?;
        writeln!(f, "Period: {} to {}", self.start_time, self.end_time)?;
        writeln!(f, "Duration: {:.2} days", self.duration_days)?;
        writeln!(f, "")?;
        writeln!(f, "Initial Balance: ${:.2}", self.initial_balance)?;
        writeln!(f, "Current Balance: ${:.2}", self.current_balance)?;
        writeln!(f, "Unrealized P&L: ${:.2}", self.unrealized_pnl)?;
        writeln!(f, "Realized P&L: ${:.2}", self.realized_pnl)?;
        writeln!(f, "Funding P&L: ${:.2}", self.funding_pnl)?;
        writeln!(f, "Total P&L: ${:.2}", self.total_pnl)?;
        writeln!(f, "Total Fees: ${:.2}", self.total_fees)?;
        writeln!(f, "")?;
        writeln!(f, "Total Equity: ${:.2}", self.total_equity)?;
        writeln!(f, "Total Return: ${:.2} ({:.2}%)", self.total_return, self.total_return_pct)?;
        writeln!(f, "Annualized Return: {:.2}%", self.annualized_return)?;
        writeln!(f, "")?;
        writeln!(f, "Trade Count: {}", self.trade_count)?;
        writeln!(f, "Winning Trades: {} ({:.2}%)", self.winning_trades, self.win_rate)?;
        writeln!(f, "Losing Trades: {}", self.losing_trades)?;
        writeln!(f, "Maximum Drawdown: ${:.2} ({:.2}%)", self.max_drawdown, self.max_drawdown_pct)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    // Mock TradingStrategy for testing
    struct MockStrategy {
        name: String,
        signals: HashMap<String, Signal>,
    }
    
    impl TradingStrategy for MockStrategy {
        fn name(&self) -> &str {
            &self.name
        }
        
        fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
            // Simple strategy: buy when price increases, sell when price decreases
            let symbol = &data.symbol;
            let signal = self.signals.get(symbol);
            
            match signal {
                Some(signal) => {
                    match signal.direction {
                        SignalDirection::Buy => {
                            Ok(vec![OrderRequest::market(symbol, OrderSide::Buy, 1.0)])
                        },
                        SignalDirection::Sell => {
                            Ok(vec![OrderRequest::market(symbol, OrderSide::Sell, 1.0)])
                        },
                        _ => Ok(vec![]),
                    }
                },
                None => Ok(vec![]),
            }
        }
        
        fn on_order_fill(&mut self, _fill: &OrderResult) -> Result<(), String> {
            Ok(())
        }
        
        fn on_funding_payment(&mut self, _payment: &FundingPayment) -> Result<(), String> {
            Ok(())
        }
        
        fn get_current_signals(&self) -> HashMap<String, Signal> {
            self.signals.clone()
        }
    }
    
    #[test]
    fn test_paper_trading_engine_creation() {
        let slippage_config = SlippageConfig::default();
        let engine = PaperTradingEngine::new(10000.0, slippage_config);
        
        assert_eq!(engine.simulated_balance, 10000.0);
        assert!(engine.simulated_positions.is_empty());
        assert!(engine.order_history.is_empty());
        assert!(engine.active_orders.is_empty());
    }
    
    #[test]
    fn test_market_data_update() {
        let slippage_config = SlippageConfig::default();
        let mut engine = PaperTradingEngine::new(10000.0, slippage_config);
        
        let now = Utc::now().with_timezone(&FixedOffset::east(0));
        let market_data = MarketData::new("BTC", 50000.0, 49990.0, 50010.0, 100.0, now);
        
        // Add a position first
        let position = Position::new("BTC", 1.0, 49000.0, 49000.0, now);
        engine.simulated_positions.insert("BTC".to_string(), position);
        
        // Update market data
        engine.update_market_data(market_data).unwrap();
        
        // Check that position price was updated
        let updated_position = engine.simulated_positions.get("BTC").unwrap();
        assert_eq!(updated_position.current_price, 50000.0);
        assert_eq!(updated_position.unrealized_pnl, 1000.0); // 1 BTC * (50000 - 49000)
    }
    
    #[test]
    fn test_market_order_execution() {
        let slippage_config = SlippageConfig {
            base_slippage_pct: 0.0, // No slippage for testing
            volume_impact_factor: 0.0,
            volatility_impact_factor: 0.0,
            random_slippage_max_pct: 0.0,
            simulated_latency_ms: 0,
            use_order_book: false,
            max_slippage_pct: 0.0,
        };
        
        let mut engine = PaperTradingEngine::new(10000.0, slippage_config);
        
        let now = Utc::now().with_timezone(&FixedOffset::east(0));
        let market_data = MarketData::new("BTC", 50000.0, 49990.0, 50010.0, 100.0, now);
        
        // Add market data
        engine.market_data_cache.insert("BTC".to_string(), market_data);
        
        // Create a market buy order
        let order = OrderRequest::market("BTC", OrderSide::Buy, 0.1);
        
        // Execute the order
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(engine.execute_order(order)).unwrap();
        
        // Check the result
        assert_eq!(result.status, OrderStatus::Filled);
        assert_eq!(result.filled_quantity, 0.1);
        assert!(result.average_price.is_some());
        assert!(result.fees.is_some());
        
        // Check the position
        let position = engine.simulated_positions.get("BTC").unwrap();
        assert_eq!(position.size, 0.1);
        assert_eq!(position.entry_price, 50010.0); // Buy at ask price
        
        // Check the balance (10000 - (0.1 * 50010) - fees)
        let fees = 0.1 * 50010.0 * engine.taker_fee;
        assert_eq!(engine.simulated_balance, 10000.0 - (0.1 * 50010.0) - fees);
    }
    
    #[test]
    fn test_limit_order_execution() {
        let slippage_config = SlippageConfig::default();
        let mut engine = PaperTradingEngine::new(10000.0, slippage_config);
        
        let now = Utc::now().with_timezone(&FixedOffset::east(0));
        let market_data = MarketData::new("BTC", 50000.0, 49990.0, 50010.0, 100.0, now);
        
        // Add market data
        engine.market_data_cache.insert("BTC".to_string(), market_data.clone());
        
        // Create a limit buy order below current ask price
        let limit_price = 49980.0;
        let order = OrderRequest::limit("BTC", OrderSide::Buy, 0.1, limit_price);
        
        // Execute the order (should be added to active orders)
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(engine.execute_order(order)).unwrap();
        
        // Check the result
        assert_eq!(result.status, OrderStatus::Submitted);
        assert_eq!(engine.active_orders.len(), 1);
        
        // Update market data with lower ask price that triggers the limit order
        let new_market_data = MarketData::new("BTC", 49970.0, 49960.0, 49980.0, 100.0, now);
        engine.update_market_data(new_market_data).unwrap();
        
        // Check that the order was executed
        assert_eq!(engine.active_orders.len(), 0);
        assert_eq!(engine.order_history.len(), 1);
        
        // Check the position
        let position = engine.simulated_positions.get("BTC").unwrap();
        assert_eq!(position.size, 0.1);
        assert_eq!(position.entry_price, limit_price);
    }
    
    #[test]
    fn test_position_tracking() {
        let slippage_config = SlippageConfig {
            base_slippage_pct: 0.0, // No slippage for testing
            volume_impact_factor: 0.0,
            volatility_impact_factor: 0.0,
            random_slippage_max_pct: 0.0,
            simulated_latency_ms: 0,
            use_order_book: false,
            max_slippage_pct: 0.0,
        };
        
        let mut engine = PaperTradingEngine::new(10000.0, slippage_config);
        
        let now = Utc::now().with_timezone(&FixedOffset::east(0));
        let market_data = MarketData::new("BTC", 50000.0, 49990.0, 50010.0, 100.0, now);
        
        // Add market data
        engine.market_data_cache.insert("BTC".to_string(), market_data);
        
        // Create and execute a market buy order
        let buy_order = OrderRequest::market("BTC", OrderSide::Buy, 0.1);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(engine.execute_order(buy_order)).unwrap();
        
        // Check the position
        let position = engine.simulated_positions.get("BTC").unwrap();
        assert_eq!(position.size, 0.1);
        
        // Create and execute a market sell order (partial close)
        let sell_order = OrderRequest::market("BTC", OrderSide::Sell, 0.05);
        rt.block_on(engine.execute_order(sell_order)).unwrap();
        
        // Check the position
        let position = engine.simulated_positions.get("BTC").unwrap();
        assert_eq!(position.size, 0.05);
        
        // Create and execute another market sell order (full close)
        let sell_order = OrderRequest::market("BTC", OrderSide::Sell, 0.05);
        rt.block_on(engine.execute_order(sell_order)).unwrap();
        
        // Check the position
        let position = engine.simulated_positions.get("BTC").unwrap();
        assert_eq!(position.size, 0.0);
    }
    
    #[test]
    fn test_performance_metrics() {
        let slippage_config = SlippageConfig {
            base_slippage_pct: 0.0, // No slippage for testing
            volume_impact_factor: 0.0,
            volatility_impact_factor: 0.0,
            random_slippage_max_pct: 0.0,
            simulated_latency_ms: 0,
            use_order_book: false,
            max_slippage_pct: 0.0,
        };
        
        let mut engine = PaperTradingEngine::new(10000.0, slippage_config);
        
        let now = Utc::now().with_timezone(&FixedOffset::east(0));
        
        // Add market data
        let market_data = MarketData::new("BTC", 50000.0, 49990.0, 50010.0, 100.0, now);
        engine.market_data_cache.insert("BTC".to_string(), market_data);
        
        // Execute a buy order
        let buy_order = OrderRequest::market("BTC", OrderSide::Buy, 0.1);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(engine.execute_order(buy_order)).unwrap();
        
        // Update market data with higher price
        let new_market_data = MarketData::new("BTC", 51000.0, 50990.0, 51010.0, 100.0, now);
        engine.update_market_data(new_market_data).unwrap();
        
        // Execute a sell order
        let sell_order = OrderRequest::market("BTC", OrderSide::Sell, 0.1);
        rt.block_on(engine.execute_order(sell_order)).unwrap();
        
        // Check metrics
        let metrics = engine.get_metrics();
        assert!(metrics.realized_pnl > 0.0); // Should have made profit
        assert_eq!(metrics.trade_count, 1);
        assert_eq!(metrics.winning_trades, 1);
        
        // Generate report
        let report = engine.generate_report();
        assert!(report.total_return > 0.0);
        assert!(report.total_return_pct > 0.0);
        assert!(report.win_rate > 0.0);
    }
}