use std::sync::{Arc, Mutex};
use std::time::Duration;
use chrono::{DateTime, FixedOffset, Utc};
use std::collections::HashMap;

use hyperliquid_backtester::prelude::*;
use hyperliquid_backtester::trading_mode::{TradingConfig, RiskConfig, SlippageConfig};
use hyperliquid_backtester::unified_data::{
    Position, OrderRequest, OrderResult, MarketData, 
    OrderSide, OrderType, TimeInForce, OrderStatus,
    TradingStrategy, Signal, SignalDirection
};

/// A simple moving average crossover strategy with funding rate awareness
struct FundingAwareSmaStrategy {
    name: String,
    symbol: String,
    short_period: usize,
    long_period: usize,
    prices: Vec<f64>,
    funding_rates: Vec<f64>,
    funding_threshold: f64,
    funding_weight: f64,
    position_size: f64,
    current_position: f64,
    signals: HashMap<String, Signal>,
}

impl FundingAwareSmaStrategy {
    fn new(
        symbol: &str, 
        short_period: usize, 
        long_period: usize,
        funding_threshold: f64,
        funding_weight: f64,
        position_size: f64
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
            position_size,
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
                            quantity: self.position_size * signal.strength,
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
                            quantity: self.position_size * signal.strength,
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    init_logger();
    
    println!("Hyperliquid Paper Trading Setup Example");
    println!("======================================");
    
    // Step 1: Create trading configuration
    println!("\n1. Creating Trading Configuration");
    println!("--------------------------------");
    
    // Create risk configuration
    let risk_config = RiskConfig {
        max_position_size_pct: 0.1,     // 10% of portfolio
        max_daily_loss_pct: 0.02,       // 2% max daily loss
        stop_loss_pct: 0.05,            // 5% stop loss
        take_profit_pct: 0.1,           // 10% take profit
        max_leverage: 3.0,              // 3x max leverage
        max_concentration_pct: 0.25,    // 25% max concentration
        max_position_correlation: 0.7,  // 70% max correlation
        volatility_sizing_factor: 0.5,  // 50% volatility impact
        max_portfolio_volatility_pct: 0.05, // 5% max portfolio volatility
        max_drawdown_pct: 0.1,          // 10% max drawdown
    };
    
    // Create slippage configuration
    let slippage_config = SlippageConfig {
        base_slippage_pct: 0.0005,      // 0.05% base slippage
        volume_impact_factor: 0.1,      // Volume impact factor
        volatility_impact_factor: 0.2,  // Volatility impact factor
        random_slippage_max_pct: 0.001, // 0.1% max random component
        simulated_latency_ms: 500,      // 500ms simulated latency
    };
    
    // Create trading configuration
    let trading_config = TradingConfig::new(10000.0)  // $10,000 initial balance
        .with_risk_config(risk_config)
        .with_slippage_config(slippage_config)
        .with_parameter("max_open_orders", "5")
        .with_parameter("enable_trailing_stop", "true");
    
    println!("✅ Trading configuration created:");
    println!("   - Initial balance: ${:.2}", trading_config.initial_balance);
    println!("   - Max position size: {:.1}%", trading_config.risk_config.as_ref().unwrap().max_position_size_pct * 100.0);
    println!("   - Max daily loss: {:.1}%", trading_config.risk_config.as_ref().unwrap().max_daily_loss_pct * 100.0);
    println!("   - Base slippage: {:.3}%", trading_config.slippage_config.as_ref().unwrap().base_slippage_pct * 100.0);
    
    // Step 2: Create real-time data stream
    println!("\n2. Setting Up Real-Time Data Stream");
    println!("--------------------------------");
    
    let data_stream = RealTimeDataStream::new().await?;
    let data_stream = Arc::new(Mutex::new(data_stream));
    
    // Connect to data stream
    {
        let mut stream = data_stream.lock().unwrap();
        stream.connect().await?;
        println!("✅ Connected to real-time data stream");
        
        // Subscribe to market data
        stream.subscribe_ticker("BTC").await?;
        stream.subscribe_order_book("BTC").await?;
        stream.subscribe_funding_rate("BTC").await?;
        println!("✅ Subscribed to BTC market data");
    }
    
    // Step 3: Create paper trading engine
    println!("\n3. Creating Paper Trading Engine");
    println!("--------------------------------");
    
    let mut paper_engine = PaperTradingEngine::new(
        trading_config.initial_balance,
        trading_config.slippage_config.unwrap_or_default()
    );
    
    // Set the real-time data stream
    paper_engine.set_real_time_data(data_stream.clone());
    
    // Set risk manager
    if let Some(risk_config) = trading_config.risk_config {
        paper_engine.set_risk_config(risk_config);
        println!("✅ Risk manager configured");
    }
    
    println!("✅ Paper trading engine created");
    
    // Step 4: Create trading strategy
    println!("\n4. Creating Trading Strategy");
    println!("---------------------------");
    
    // Create a funding-aware SMA strategy
    let strategy = FundingAwareSmaStrategy::new(
        "BTC",      // Symbol
        10,         // Short period
        30,         // Long period
        0.0001,     // Funding threshold
        0.5,        // Funding weight
        0.1         // Position size (10% of portfolio)
    );
    
    println!("✅ Created strategy: {}", strategy.name());
    println!("   - Symbol: BTC");
    println!("   - Short/Long periods: 10/30");
    println!("   - Funding threshold: 0.01%");
    println!("   - Funding weight: 50%");
    
    // Step 5: Start paper trading simulation
    println!("\n5. Starting Paper Trading Simulation");
    println!("----------------------------------");
    
    let strategy_box: Box<dyn TradingStrategy> = Box::new(strategy);
    
    // Start paper trading in a separate task
    let paper_engine_arc = Arc::new(Mutex::new(paper_engine));
    let paper_engine_for_task = paper_engine_arc.clone();
    
    let task_handle = tokio::spawn(async move {
        let mut engine = paper_engine_for_task.lock().unwrap();
        if let Err(e) = engine.start_simulation(strategy_box).await {
            eprintln!("Error in paper trading simulation: {}", e);
        }
    });
    
    // Let the simulation run for a while
    println!("Paper trading simulation started. Running for 30 seconds...");
    println!("(In a real application, this would run continuously)");
    
    // Simulate running for 30 seconds
    tokio::time::sleep(Duration::from_secs(30)).await;
    
    // Step 6: Stop simulation and analyze results
    println!("\n6. Stopping Simulation and Analyzing Results");
    println!("------------------------------------------");
    
    // Stop the simulation
    {
        let mut engine = paper_engine_arc.lock().unwrap();
        engine.stop_simulation();
        println!("✅ Simulation stopped");
    }
    
    // Wait for the task to complete
    let _ = task_handle.await;
    
    // Get the final results
    let report = {
        let engine = paper_engine_arc.lock().unwrap();
        engine.generate_report()
    };
    
    // Print the report
    println!("\nPaper Trading Results:");
    println!("---------------------");
    println!("{}", report);
    
    // Print positions
    let positions = {
        let engine = paper_engine_arc.lock().unwrap();
        engine.get_positions().clone()
    };
    
    println!("\nFinal Positions:");
    if positions.is_empty() {
        println!("No open positions");
    } else {
        for (symbol, position) in positions {
            println!("{}: {} @ ${} (PnL: ${:.2})", 
                symbol, 
                position.size, 
                position.current_price,
                position.total_pnl()
            );
        }
    }
    
    // Print trade history
    let trade_log = {
        let engine = paper_engine_arc.lock().unwrap();
        engine.get_trade_log().clone()
    };
    
    println!("\nTrade History:");
    if trade_log.is_empty() {
        println!("No trades executed");
    } else {
        for (i, trade) in trade_log.iter().enumerate() {
            println!("{}. {} {} {} @ ${} (Fees: ${:.2})", 
                i + 1,
                trade.timestamp.format("%Y-%m-%d %H:%M:%S"),
                trade.side,
                trade.quantity,
                trade.price,
                trade.fees
            );
        }
    }
    
    // Step 7: Export results
    println!("\n7. Exporting Results");
    println!("-------------------");
    
    let csv_export = {
        let engine = paper_engine_arc.lock().unwrap();
        engine.export_to_csv()?
    };
    
    println!("✅ Results exported to CSV format");
    println!("   (In a real application, this would be saved to a file)");
    
    println!("\nExample completed successfully!");
    
    Ok(())
}