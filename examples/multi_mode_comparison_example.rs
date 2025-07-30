use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, FixedOffset, Utc};
use tokio::time::{sleep, Duration};

use hyperliquid_backtest::prelude::*;
use hyperliquid_backtest::trading_mode::{
    TradingMode, TradingModeManager, TradingConfig, RiskConfig, SlippageConfig, ApiConfig
};
use hyperliquid_backtest::unified_data::{
    Position, OrderRequest, OrderResult, MarketData, 
    OrderSide, OrderType, TimeInForce, OrderStatus,
    TradingStrategy, Signal, SignalDirection, FundingPayment
};
use hyperliquid_backtest::logging::init_logger;

/// A simple strategy factory that creates different strategy variants
struct StrategyFactory;

impl StrategyFactory {
    /// Create a simple SMA crossover strategy
    fn create_sma_strategy(symbol: &str, short_period: usize, long_period: usize) -> Box<dyn TradingStrategy> {
        Box::new(SmaStrategy::new(symbol, short_period, long_period))
    }
    
    /// Create a funding-aware SMA crossover strategy
    fn create_funding_aware_sma_strategy(
        symbol: &str, 
        short_period: usize, 
        long_period: usize,
        funding_threshold: f64,
        funding_weight: f64
    ) -> Box<dyn TradingStrategy> {
        Box::new(FundingAwareSmaStrategy::new(
            symbol, short_period, long_period, funding_threshold, funding_weight
        ))
    }
    
    /// Create a pure funding arbitrage strategy
    fn create_funding_arbitrage_strategy(
        symbol: &str,
        funding_threshold: f64
    ) -> Box<dyn TradingStrategy> {
        Box::new(FundingArbitrageStrategy::new(symbol, funding_threshold))
    }
}

/// Simple SMA Crossover Strategy
struct SmaStrategy {
    name: String,
    symbol: String,
    short_period: usize,
    long_period: usize,
    prices: Vec<f64>,
    current_position: f64,
    signals: HashMap<String, Signal>,
}

impl SmaStrategy {
    fn new(symbol: &str, short_period: usize, long_period: usize) -> Self {
        Self {
            name: format!("SMA {}/{}", short_period, long_period),
            symbol: symbol.to_string(),
            short_period,
            long_period,
            prices: Vec::new(),
            current_position: 0.0,
            signals: HashMap::new(),
        }
    }
    
    fn calculate_sma(&self, period: usize) -> Option<f64> {
        if self.prices.len() < period {
            return None;
        }
        
        let sum: f64 = self.prices.iter().rev().take(period).sum();
        Some(sum / period as f64)
    }
}

impl TradingStrategy for SmaStrategy {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
        if data.symbol != self.symbol {
            return Ok(Vec::new());
        }
        
        // Store the price
        self.prices.push(data.price);
        
        // Keep only necessary history
        let max_period = self.short_period.max(self.long_period);
        if self.prices.len() > max_period * 2 {
            self.prices.remove(0);
        }
        
        // Calculate SMAs
        let short_sma = self.calculate_sma(self.short_period);
        let long_sma = self.calculate_sma(self.long_period);
        
        // Generate signals
        let mut orders = Vec::new();
        
        if let (Some(short), Some(long)) = (short_sma, long_sma) {
            let now = Utc::now().with_timezone(&FixedOffset::east(0));
            
            // Crossover logic
            let signal_direction = if short > long {
                // Short SMA above long SMA - bullish
                SignalDirection::Buy
            } else if short < long {
                // Short SMA below long SMA - bearish
                SignalDirection::Sell
            } else {
                SignalDirection::Neutral
            };
            
            // Create signal
            let signal = Signal {
                symbol: self.symbol.clone(),
                direction: signal_direction,
                strength: 1.0,
                timestamp: now,
                metadata: HashMap::new(),
            };
            
            // Check if signal changed
            let previous_signal = self.signals.get(&self.symbol);
            let signal_changed = match previous_signal {
                Some(prev) => prev.direction != signal.direction,
                None => signal.direction != SignalDirection::Neutral,
            };
            
            // Store the new signal
            self.signals.insert(self.symbol.clone(), signal.clone());
            
            // Generate orders on signal change
            if signal_changed {
                match signal.direction {
                    SignalDirection::Buy => {
                        // Close any existing short position
                        if self.current_position < 0.0 {
                            orders.push(OrderRequest {
                                symbol: self.symbol.clone(),
                                side: OrderSide::Buy,
                                order_type: OrderType::Market,
                                quantity: self.current_position.abs(),
                                price: None,
                                reduce_only: true,
                                time_in_force: TimeInForce::ImmediateOrCancel,
                                client_order_id: Some(format!("close_short_{}", now.timestamp())),
                                metadata: HashMap::new(),
                            });
                        }
                        
                        // Open long position
                        orders.push(OrderRequest {
                            symbol: self.symbol.clone(),
                            side: OrderSide::Buy,
                            order_type: OrderType::Market,
                            quantity: 1.0, // Fixed position size
                            price: None,
                            reduce_only: false,
                            time_in_force: TimeInForce::ImmediateOrCancel,
                            client_order_id: Some(format!("open_long_{}", now.timestamp())),
                            metadata: HashMap::new(),
                        });
                    },
                    SignalDirection::Sell => {
                        // Close any existing long position
                        if self.current_position > 0.0 {
                            orders.push(OrderRequest {
                                symbol: self.symbol.clone(),
                                side: OrderSide::Sell,
                                order_type: OrderType::Market,
                                quantity: self.current_position,
                                price: None,
                                reduce_only: true,
                                time_in_force: TimeInForce::ImmediateOrCancel,
                                client_order_id: Some(format!("close_long_{}", now.timestamp())),
                                metadata: HashMap::new(),
                            });
                        }
                        
                        // Open short position
                        orders.push(OrderRequest {
                            symbol: self.symbol.clone(),
                            side: OrderSide::Sell,
                            order_type: OrderType::Market,
                            quantity: 1.0, // Fixed position size
                            price: None,
                            reduce_only: false,
                            time_in_force: TimeInForce::ImmediateOrCancel,
                            client_order_id: Some(format!("open_short_{}", now.timestamp())),
                            metadata: HashMap::new(),
                        });
                    },
                    SignalDirection::Neutral => {
                        // Close any existing position
                        if self.current_position > 0.0 {
                            orders.push(OrderRequest {
                                symbol: self.symbol.clone(),
                                side: OrderSide::Sell,
                                order_type: OrderType::Market,
                                quantity: self.current_position,
                                price: None,
                                reduce_only: true,
                                time_in_force: TimeInForce::ImmediateOrCancel,
                                client_order_id: Some(format!("close_position_{}", now.timestamp())),
                                metadata: HashMap::new(),
                            });
                        } else if self.current_position < 0.0 {
                            orders.push(OrderRequest {
                                symbol: self.symbol.clone(),
                                side: OrderSide::Buy,
                                order_type: OrderType::Market,
                                quantity: self.current_position.abs(),
                                price: None,
                                reduce_only: true,
                                time_in_force: TimeInForce::ImmediateOrCancel,
                                client_order_id: Some(format!("close_position_{}", now.timestamp())),
                                metadata: HashMap::new(),
                            });
                        }
                    },
                    _ => {}
                }
            }
        }
        
        Ok(orders)
    }
    
    fn on_order_fill(&mut self, fill: &OrderResult) -> Result<(), String> {
        if fill.symbol != self.symbol {
            return Ok(());
        }
        
        // Update position based on fill
        match fill.side {
            OrderSide::Buy => {
                self.current_position += fill.filled_quantity;
            },
            OrderSide::Sell => {
                self.current_position -= fill.filled_quantity;
            },
        }
        
        Ok(())
    }
    
    fn on_funding_payment(&mut self, _payment: &FundingPayment) -> Result<(), String> {
        // Basic SMA strategy doesn't react to funding payments
        Ok(())
    }
    
    fn get_current_signals(&self) -> HashMap<String, Signal> {
        self.signals.clone()
    }
}/// Fund
ing-Aware SMA Crossover Strategy
struct FundingAwareSmaStrategy {
    name: String,
    symbol: String,
    short_period: usize,
    long_period: usize,
    prices: Vec<f64>,
    funding_rates: Vec<f64>,
    funding_threshold: f64,
    funding_weight: f64,
    current_position: f64,
    signals: HashMap<String, Signal>,
}

impl FundingAwareSmaStrategy {
    fn new(
        symbol: &str, 
        short_period: usize, 
        long_period: usize,
        funding_threshold: f64,
        funding_weight: f64
    ) -> Self {
        Self {
            name: format!("Funding-Aware SMA {}/{}", short_period, long_period),
            symbol: symbol.to_string(),
            short_period,
            long_period,
            prices: Vec::new(),
            funding_rates: Vec::new(),
            funding_threshold,
            funding_weight,
            current_position: 0.0,
            signals: HashMap::new(),
        }
    }
    
    fn calculate_sma(&self, period: usize) -> Option<f64> {
        if self.prices.len() < period {
            return None;
        }
        
        let sum: f64 = self.prices.iter().rev().take(period).sum();
        Some(sum / period as f64)
    }
    
    fn get_current_funding_bias(&self) -> f64 {
        if self.funding_rates.is_empty() {
            return 0.0;
        }
        
        // Get the latest funding rate
        let latest_funding = *self.funding_rates.last().unwrap();
        
        // Calculate funding bias
        // Positive funding rate favors short positions (negative bias)
        // Negative funding rate favors long positions (positive bias)
        if latest_funding.abs() < self.funding_threshold {
            return 0.0; // No significant bias if funding rate is small
        }
        
        -latest_funding.signum() * self.funding_weight
    }
}

impl TradingStrategy for FundingAwareSmaStrategy {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
        if data.symbol != self.symbol {
            return Ok(Vec::new());
        }
        
        // Store the price
        self.prices.push(data.price);
        
        // Store funding rate if available
        if let Some(funding_rate) = data.funding_rate {
            self.funding_rates.push(funding_rate);
        }
        
        // Keep only necessary history
        let max_period = self.short_period.max(self.long_period);
        if self.prices.len() > max_period * 2 {
            self.prices.remove(0);
        }
        
        if self.funding_rates.len() > 10 {
            self.funding_rates.remove(0);
        }
        
        // Calculate SMAs
        let short_sma = self.calculate_sma(self.short_period);
        let long_sma = self.calculate_sma(self.long_period);
        
        // Generate signals
        let mut orders = Vec::new();
        
        if let (Some(short), Some(long)) = (short_sma, long_sma) {
            let now = Utc::now().with_timezone(&FixedOffset::east(0));
            
            // Get funding bias (-1.0 to 1.0)
            let funding_bias = self.get_current_funding_bias();
            
            // Crossover logic with funding bias
            let signal_direction = if short > long && funding_bias >= 0.0 {
                // Strong buy signal: short SMA above long SMA and funding favors long
                SignalDirection::Buy
            } else if short < long && funding_bias <= 0.0 {
                // Strong sell signal: short SMA below long SMA and funding favors short
                SignalDirection::Sell
            } else if short > long && funding_bias < 0.0 {
                // Mixed signals: technical is bullish but funding favors short
                if funding_bias.abs() > 0.7 {
                    // Strong funding bias overrides technical
                    SignalDirection::Sell
                } else {
                    // Technical signal prevails but with reduced strength
                    SignalDirection::Buy
                }
            } else if short < long && funding_bias > 0.0 {
                // Mixed signals: technical is bearish but funding favors long
                if funding_bias.abs() > 0.7 {
                    // Strong funding bias overrides technical
                    SignalDirection::Buy
                } else {
                    // Technical signal prevails but with reduced strength
                    SignalDirection::Sell
                }
            } else {
                SignalDirection::Neutral
            };
            
            // Create signal
            let signal = Signal {
                symbol: self.symbol.clone(),
                direction: signal_direction,
                strength: 1.0 - funding_bias.abs() * 0.3, // Reduce strength when funding and technical conflict
                timestamp: now,
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("short_sma".to_string(), short.to_string());
                    metadata.insert("long_sma".to_string(), long.to_string());
                    metadata.insert("funding_bias".to_string(), funding_bias.to_string());
                    metadata
                },
            };
            
            // Check if signal changed
            let previous_signal = self.signals.get(&self.symbol);
            let signal_changed = match previous_signal {
                Some(prev) => prev.direction != signal.direction,
                None => signal.direction != SignalDirection::Neutral,
            };
            
            // Store the new signal
            self.signals.insert(self.symbol.clone(), signal.clone());
            
            // Generate orders on signal change
            if signal_changed {
                match signal.direction {
                    SignalDirection::Buy => {
                        // Close any existing short position
                        if self.current_position < 0.0 {
                            orders.push(OrderRequest {
                                symbol: self.symbol.clone(),
                                side: OrderSide::Buy,
                                order_type: OrderType::Market,
                                quantity: self.current_position.abs(),
                                price: None,
                                reduce_only: true,
                                time_in_force: TimeInForce::ImmediateOrCancel,
                                client_order_id: Some(format!("close_short_{}", now.timestamp())),
                                metadata: HashMap::new(),
                            });
                        }
                        
                        // Open long position
                        orders.push(OrderRequest {
                            symbol: self.symbol.clone(),
                            side: OrderSide::Buy,
                            order_type: OrderType::Market,
                            quantity: 1.0 * signal.strength, // Position size scaled by signal strength
                            price: None,
                            reduce_only: false,
                            time_in_force: TimeInForce::ImmediateOrCancel,
                            client_order_id: Some(format!("open_long_{}", now.timestamp())),
                            metadata: HashMap::new(),
                        });
                    },
                    SignalDirection::Sell => {
                        // Close any existing long position
                        if self.current_position > 0.0 {
                            orders.push(OrderRequest {
                                symbol: self.symbol.clone(),
                                side: OrderSide::Sell,
                                order_type: OrderType::Market,
                                quantity: self.current_position,
                                price: None,
                                reduce_only: true,
                                time_in_force: TimeInForce::ImmediateOrCancel,
                                client_order_id: Some(format!("close_long_{}", now.timestamp())),
                                metadata: HashMap::new(),
                            });
                        }
                        
                        // Open short position
                        orders.push(OrderRequest {
                            symbol: self.symbol.clone(),
                            side: OrderSide::Sell,
                            order_type: OrderType::Market,
                            quantity: 1.0 * signal.strength, // Position size scaled by signal strength
                            price: None,
                            reduce_only: false,
                            time_in_force: TimeInForce::ImmediateOrCancel,
                            client_order_id: Some(format!("open_short_{}", now.timestamp())),
                            metadata: HashMap::new(),
                        });
                    },
                    SignalDirection::Neutral => {
                        // Close any existing position
                        if self.current_position > 0.0 {
                            orders.push(OrderRequest {
                                symbol: self.symbol.clone(),
                                side: OrderSide::Sell,
                                order_type: OrderType::Market,
                                quantity: self.current_position,
                                price: None,
                                reduce_only: true,
                                time_in_force: TimeInForce::ImmediateOrCancel,
                                client_order_id: Some(format!("close_position_{}", now.timestamp())),
                                metadata: HashMap::new(),
                            });
                        } else if self.current_position < 0.0 {
                            orders.push(OrderRequest {
                                symbol: self.symbol.clone(),
                                side: OrderSide::Buy,
                                order_type: OrderType::Market,
                                quantity: self.current_position.abs(),
                                price: None,
                                reduce_only: true,
                                time_in_force: TimeInForce::ImmediateOrCancel,
                                client_order_id: Some(format!("close_position_{}", now.timestamp())),
                                metadata: HashMap::new(),
                            });
                        }
                    },
                    _ => {}
                }
            }
        }
        
        Ok(orders)
    }
    
    fn on_order_fill(&mut self, fill: &OrderResult) -> Result<(), String> {
        if fill.symbol != self.symbol {
            return Ok(());
        }
        
        // Update position based on fill
        match fill.side {
            OrderSide::Buy => {
                self.current_position += fill.filled_quantity;
            },
            OrderSide::Sell => {
                self.current_position -= fill.filled_quantity;
            },
        }
        
        Ok(())
    }
    
    fn on_funding_payment(&mut self, payment: &FundingPayment) -> Result<(), String> {
        // Log funding payment
        println!("Funding payment received: {} {} (rate: {})", 
            payment.symbol, payment.amount, payment.rate);
        Ok(())
    }
    
    fn get_current_signals(&self) -> HashMap<String, Signal> {
        self.signals.clone()
    }
}/
// Pure Funding Arbitrage Strategy
struct FundingArbitrageStrategy {
    name: String,
    symbol: String,
    funding_threshold: f64,
    current_position: f64,
    signals: HashMap<String, Signal>,
    last_funding_rate: Option<f64>,
}

impl FundingArbitrageStrategy {
    fn new(symbol: &str, funding_threshold: f64) -> Self {
        Self {
            name: format!("Funding Arbitrage (threshold: {}%)", funding_threshold * 100.0),
            symbol: symbol.to_string(),
            funding_threshold,
            current_position: 0.0,
            signals: HashMap::new(),
            last_funding_rate: None,
        }
    }
}

impl TradingStrategy for FundingArbitrageStrategy {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
        if data.symbol != self.symbol {
            return Ok(Vec::new());
        }
        
        // Only react to funding rate changes
        if let Some(funding_rate) = data.funding_rate {
            // Check if funding rate has changed significantly
            let funding_changed = match self.last_funding_rate {
                Some(last_rate) => (funding_rate - last_rate).abs() > self.funding_threshold * 0.5,
                None => true,
            };
            
            // Update last funding rate
            self.last_funding_rate = Some(funding_rate);
            
            // Only generate orders if funding rate exceeds threshold or has changed significantly
            if funding_rate.abs() > self.funding_threshold || funding_changed {
                let now = Utc::now().with_timezone(&FixedOffset::east(0));
                let mut orders = Vec::new();
                
                // Determine position side based on funding rate
                // For funding arbitrage:
                // - When funding rate is positive, go short (collect funding)
                // - When funding rate is negative, go long (collect funding)
                let target_position = if funding_rate.abs() <= self.funding_threshold {
                    // Funding rate too small, close position
                    0.0
                } else if funding_rate > 0.0 {
                    // Positive funding rate, go short
                    -1.0
                } else {
                    // Negative funding rate, go long
                    1.0
                };
                
                // Check if we need to change position
                if (target_position - self.current_position).abs() > 0.01 {
                    // Close existing position if needed
                    if self.current_position != 0.0 {
                        let close_side = if self.current_position > 0.0 {
                            OrderSide::Sell
                        } else {
                            OrderSide::Buy
                        };
                        
                        orders.push(OrderRequest {
                            symbol: self.symbol.clone(),
                            side: close_side,
                            order_type: OrderType::Market,
                            quantity: self.current_position.abs(),
                            price: None,
                            reduce_only: true,
                            time_in_force: TimeInForce::ImmediateOrCancel,
                            client_order_id: Some(format!("close_position_{}", now.timestamp())),
                            metadata: HashMap::new(),
                        });
                    }
                    
                    // Open new position if target is not zero
                    if target_position != 0.0 {
                        let open_side = if target_position > 0.0 {
                            OrderSide::Buy
                        } else {
                            OrderSide::Sell
                        };
                        
                        orders.push(OrderRequest {
                            symbol: self.symbol.clone(),
                            side: open_side,
                            order_type: OrderType::Market,
                            quantity: target_position.abs(),
                            price: None,
                            reduce_only: false,
                            time_in_force: TimeInForce::ImmediateOrCancel,
                            client_order_id: Some(format!("open_position_{}", now.timestamp())),
                            metadata: HashMap::new(),
                        });
                    }
                    
                    // Update signal
                    let direction = if target_position > 0.0 {
                        SignalDirection::Buy
                    } else if target_position < 0.0 {
                        SignalDirection::Sell
                    } else {
                        SignalDirection::Neutral
                    };
                    
                    self.signals.insert(
                        self.symbol.clone(),
                        Signal {
                            symbol: self.symbol.clone(),
                            direction,
                            strength: funding_rate.abs() / self.funding_threshold,
                            timestamp: now,
                            metadata: {
                                let mut metadata = HashMap::new();
                                metadata.insert("funding_rate".to_string(), funding_rate.to_string());
                                metadata
                            },
                        }
                    );
                    
                    return Ok(orders);
                }
            }
        }
        
        Ok(Vec::new())
    }
    
    fn on_order_fill(&mut self, fill: &OrderResult) -> Result<(), String> {
        if fill.symbol != self.symbol {
            return Ok(());
        }
        
        // Update position based on fill
        match fill.side {
            OrderSide::Buy => {
                self.current_position += fill.filled_quantity;
            },
            OrderSide::Sell => {
                self.current_position -= fill.filled_quantity;
            },
        }
        
        Ok(())
    }
    
    fn on_funding_payment(&mut self, payment: &FundingPayment) -> Result<(), String> {
        // Log funding payment
        println!("Funding payment received: {} {} (rate: {})", 
            payment.symbol, payment.amount, payment.rate);
        Ok(())
    }
    
    fn get_current_signals(&self) -> HashMap<String, Signal> {
        self.signals.clone()
    }
}/// 
Adapter to use TradingStrategy with rs-backtester
struct BacktestStrategyAdapter {
    strategy: Box<dyn TradingStrategy>,
    current_index: usize,
}

impl BacktestStrategyAdapter {
    fn new(strategy: Box<dyn TradingStrategy>) -> Self {
        Self {
            strategy,
            current_index: 0,
        }
    }
}

impl rs_backtester::strategies::Strategy for BacktestStrategyAdapter {
    fn next(&mut self, ctx: &mut rs_backtester::strategies::Context, _: &mut rs_backtester::strategies::Broker) {
        let index = ctx.index();
        self.current_index = index;
        
        // Convert rs-backtester data to MarketData
        let data = ctx.data();
        let timestamp = data.datetime[index];
        
        // Create market data with funding rate if available
        let mut market_data = MarketData::new(
            &data.ticker,
            data.close[index],
            data.low[index],
            data.high[index],
            data.volume[index],
            timestamp,
        );
        
        // Add funding rate if available in custom fields
        if let Some(custom) = ctx.custom() {
            if let Some(funding_rates) = custom.get("funding_rates") {
                if let Some(funding_rate) = funding_rates.get(index) {
                    market_data = market_data.with_funding_rate(
                        *funding_rate,
                        timestamp + chrono::Duration::hours(8)
                    );
                }
            }
        }
        
        // Process market data with strategy
        if let Ok(orders) = self.strategy.on_market_data(&market_data) {
            // Convert orders to rs-backtester actions
            for order in orders {
                match order.side {
                    OrderSide::Buy => {
                        ctx.entry_qty(order.quantity);
                    },
                    OrderSide::Sell => {
                        ctx.entry_qty(-order.quantity);
                    },
                }
            }
        }
    }
}

/// Run backtest for a strategy
async fn run_backtest(
    symbol: &str,
    strategy: Box<dyn TradingStrategy>,
    initial_balance: f64
) -> Result<FundingReport, Box<dyn std::error::Error>> {
    // Fetch historical data
    let start_time = chrono::Utc::now() - chrono::Duration::days(30);
    let end_time = chrono::Utc::now();
    
    let data = HyperliquidData::fetch(
        symbol,
        "1h",
        start_time.timestamp() as u64,
        end_time.timestamp() as u64
    ).await?;
    
    // Create backtest
    let mut backtest = HyperliquidBacktest::new(
        data,
        Box::new(BacktestStrategyAdapter::new(strategy)),
        initial_balance,
        HyperliquidCommission::default(),
    )?;
    
    // Run backtest with funding calculations
    backtest.calculate_with_funding()?;
    
    // Get report
    let report = backtest.funding_report()?;
    
    Ok(report)
}

/// Run paper trading for a strategy
async fn run_paper_trading(
    symbol: &str,
    strategy: Box<dyn TradingStrategy>,
    initial_balance: f64,
    duration_secs: u64
) -> Result<PaperTradingReport, Box<dyn std::error::Error>> {
    // Create slippage configuration
    let slippage_config = SlippageConfig {
        base_slippage_pct: 0.0005,      // 0.05% base slippage
        volume_impact_factor: 0.1,      // Volume impact factor
        volatility_impact_factor: 0.2,  // Volatility impact factor
        random_slippage_max_pct: 0.001, // 0.1% max random component
        simulated_latency_ms: 500,      // 500ms simulated latency
    };
    
    // Create paper trading engine
    let mut paper_engine = PaperTradingEngine::new(initial_balance, slippage_config);
    
    // Create real-time data stream
    let data_stream = RealTimeDataStream::new().await?;
    let data_stream = Arc::new(Mutex::new(data_stream));
    
    // Set the real-time data stream in the paper trading engine
    paper_engine.set_real_time_data(data_stream.clone());
    
    // Subscribe to market data
    {
        let mut stream = data_stream.lock().unwrap();
        stream.connect().await?;
        stream.subscribe_ticker(symbol).await?;
        stream.subscribe_order_book(symbol).await?;
        stream.subscribe_funding_rate(symbol).await?;
    }
    
    // Start paper trading in a separate task
    let paper_engine_arc = Arc::new(Mutex::new(paper_engine));
    let paper_engine_for_task = paper_engine_arc.clone();
    
    let task_handle = tokio::spawn(async move {
        let mut engine = paper_engine_for_task.lock().unwrap();
        if let Err(e) = engine.start_simulation(strategy).await {
            eprintln!("Error in paper trading simulation: {}", e);
        }
    });
    
    // Let the simulation run for the specified duration
    sleep(Duration::from_secs(duration_secs)).await;
    
    // Stop the simulation
    {
        let mut engine = paper_engine_arc.lock().unwrap();
        engine.stop_simulation();
    }
    
    // Wait for the task to complete
    let _ = task_handle.await;
    
    // Get the final results
    let report = {
        let engine = paper_engine_arc.lock().unwrap();
        engine.generate_report()
    };
    
    // Get positions
    let positions = {
        let engine = paper_engine_arc.lock().unwrap();
        engine.get_positions().clone()
    };
    
    // Get trade log
    let trade_log = {
        let engine = paper_engine_arc.lock().unwrap();
        engine.get_trade_log().clone()
    };
    
    // Create paper trading report
    let paper_report = PaperTradingReport {
        report,
        positions,
        trade_count: trade_log.len(),
    };
    
    Ok(paper_report)
}

/// Simple paper trading report structure
struct PaperTradingReport {
    report: String,
    positions: HashMap<String, Position>,
    trade_count: usize,
}

/// Strategy comparison result
struct StrategyComparisonResult {
    strategy_name: String,
    backtest_profit: f64,
    backtest_return_pct: f64,
    backtest_funding_pnl: f64,
    backtest_trading_pnl: f64,
    paper_final_equity: f64,
    paper_return_pct: f64,
    paper_trade_count: usize,
}

/// Compare strategies across different modes
async fn compare_strategies(
    symbol: &str,
    strategies: Vec<Box<dyn TradingStrategy>>,
    initial_balance: f64
) -> Result<Vec<StrategyComparisonResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();
    
    for strategy in strategies {
        let strategy_name = strategy.name().to_string();
        println!("\nEvaluating strategy: {}", strategy_name);
        
        // Run backtest
        println!("Running backtest...");
        let backtest_report = run_backtest(symbol, strategy.clone(), initial_balance).await?;
        
        // Run paper trading (short duration for example purposes)
        println!("Running paper trading simulation...");
        let paper_report = run_paper_trading(symbol, strategy, initial_balance, 10).await?;
        
        // Extract paper trading results
        let paper_final_equity = if let Some(line) = paper_report.report.lines().find(|l| l.contains("Final Equity:")) {
            if let Some(value_str) = line.split(':').nth(1) {
                value_str.trim().replace('$', "").parse::<f64>().unwrap_or(initial_balance)
            } else {
                initial_balance
            }
        } else {
            initial_balance
        };
        
        // Calculate paper trading return
        let paper_return_pct = (paper_final_equity / initial_balance - 1.0) * 100.0;
        
        // Create comparison result
        let result = StrategyComparisonResult {
            strategy_name,
            backtest_profit: backtest_report.net_profit,
            backtest_return_pct: backtest_report.net_profit / initial_balance * 100.0,
            backtest_funding_pnl: backtest_report.net_funding_pnl,
            backtest_trading_pnl: backtest_report.net_trading_pnl,
            paper_final_equity,
            paper_return_pct,
            paper_trade_count: paper_report.trade_count,
        };
        
        results.push(result);
    }
    
    Ok(results)
}#[tok
io::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    init_logger();
    
    println!("Multi-Mode Strategy Comparison Example");
    println!("=====================================");
    println!("This example demonstrates how to compare different trading strategies");
    println!("across multiple trading modes (backtest and paper trading).");
    
    // Define trading symbol and initial balance
    let symbol = "BTC";
    let initial_balance = 10000.0;
    
    println!("\nCreating strategies for comparison...");
    
    // Create strategies for comparison
    let strategies: Vec<Box<dyn TradingStrategy>> = vec![
        // Simple SMA strategy
        StrategyFactory::create_sma_strategy(symbol, 10, 20),
        
        // Funding-aware SMA strategy
        StrategyFactory::create_funding_aware_sma_strategy(symbol, 10, 20, 0.0001, 0.5),
        
        // Pure funding arbitrage strategy
        StrategyFactory::create_funding_arbitrage_strategy(symbol, 0.0002),
    ];
    
    println!("Created {} strategies for comparison", strategies.len());
    
    // Compare strategies
    println!("\nComparing strategies across trading modes...");
    let results = compare_strategies(symbol, strategies, initial_balance).await?;
    
    // Display results
    println!("\nStrategy Comparison Results:");
    println!("===========================");
    println!("{:<30} | {:<15} | {:<15} | {:<15} | {:<15} | {:<15}", 
        "Strategy", "Backtest Return", "Backtest Funding", "Backtest Trading", "Paper Return", "Paper Trades");
    println!("{:-<30} | {:-<15} | {:-<15} | {:-<15} | {:-<15} | {:-<15}", 
        "", "", "", "", "", "");
    
    for result in &results {
        println!("{:<30} | {:<15.2}% | ${:<14.2} | ${:<14.2} | {:<15.2}% | {:<15}", 
            result.strategy_name,
            result.backtest_return_pct,
            result.backtest_funding_pnl,
            result.backtest_trading_pnl,
            result.paper_return_pct,
            result.paper_trade_count
        );
    }
    
    // Find best strategy
    if let Some(best_backtest) = results.iter().max_by(|a, b| 
        a.backtest_return_pct.partial_cmp(&b.backtest_return_pct).unwrap_or(std::cmp::Ordering::Equal)) {
        println!("\nBest strategy in backtest: {} ({:.2}%)", 
            best_backtest.strategy_name, best_backtest.backtest_return_pct);
    }
    
    if let Some(best_paper) = results.iter().max_by(|a, b| 
        a.paper_return_pct.partial_cmp(&b.paper_return_pct).unwrap_or(std::cmp::Ordering::Equal)) {
        println!("Best strategy in paper trading: {} ({:.2}%)", 
            best_paper.strategy_name, best_paper.paper_return_pct);
    }
    
    // Analysis
    println!("\nAnalysis:");
    println!("--------");
    
    // Compare backtest vs paper trading performance
    let mut consistent_strategies = 0;
    for result in &results {
        if (result.backtest_return_pct > 0.0 && result.paper_return_pct > 0.0) ||
           (result.backtest_return_pct < 0.0 && result.paper_return_pct < 0.0) {
            consistent_strategies += 1;
        }
    }
    
    println!("{} out of {} strategies showed consistent performance direction between backtest and paper trading",
        consistent_strategies, results.len());
    
    // Funding impact analysis
    let mut funding_positive_impact = 0;
    for result in &results {
        if result.backtest_funding_pnl > 0.0 {
            funding_positive_impact += 1;
        }
    }
    
    println!("{} out of {} strategies benefited from funding payments in backtest",
        funding_positive_impact, results.len());
    
    // Recommendations
    println!("\nRecommendations:");
    println!("--------------");
    println!("1. For live trading deployment, consider using the strategy that performed");
    println!("   well in both backtest and paper trading modes");
    println!("2. Monitor funding rate impact closely, as it can significantly affect performance");
    println!("3. Consider running longer paper trading simulations before live deployment");
    println!("4. Implement proper risk management for any strategy deployed to live trading");
    
    println!("\nExample completed successfully!");
    
    Ok(())
}

// Clone implementation for Box<dyn TradingStrategy>
impl Clone for Box<dyn TradingStrategy> {
    fn clone(&self) -> Self {
        // This is a simplified clone implementation for the example
        // In a real application, you would need to implement proper cloning for each strategy type
        if self.name().contains("SMA") {
            if self.name().contains("Funding-Aware") {
                StrategyFactory::create_funding_aware_sma_strategy("BTC", 10, 20, 0.0001, 0.5)
            } else {
                StrategyFactory::create_sma_strategy("BTC", 10, 20)
            }
        } else {
            StrategyFactory::create_funding_arbitrage_strategy("BTC", 0.0002)
        }
    }
}