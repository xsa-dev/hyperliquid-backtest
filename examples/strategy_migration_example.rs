use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, FixedOffset, Utc};
use ethers::signers::LocalWallet;

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
//
/ Enhanced SMA Crossover Strategy with Funding Rate Awareness
struct EnhancedSmaStrategy {
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
    parameters: HashMap<String, String>,
}

impl EnhancedSmaStrategy {
    fn new(
        symbol: &str, 
        short_period: usize, 
        long_period: usize,
        funding_threshold: f64,
        funding_weight: f64,
        position_size: f64
    ) -> Self {
        Self {
            name: format!("Enhanced SMA {}/{}", short_period, long_period),
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
            parameters: {
                let mut params = HashMap::new();
                params.insert("short_period".to_string(), short_period.to_string());
                params.insert("long_period".to_string(), long_period.to_string());
                params.insert("funding_threshold".to_string(), funding_threshold.to_string());
                params.insert("funding_weight".to_string(), funding_weight.to_string());
                params.insert("position_size".to_string(), position_size.to_string());
                params
            },
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
    
    // Save strategy state to a serializable format
    fn save_state(&self) -> HashMap<String, String> {
        let mut state = HashMap::new();
        
        // Save parameters
        for (key, value) in &self.parameters {
            state.insert(format!("param_{}", key), value.clone());
        }
        
        // Save current position
        state.insert("current_position".to_string(), self.current_position.to_string());
        
        // Save latest prices (up to 5)
        for (i, price) in self.prices.iter().rev().take(5).enumerate() {
            state.insert(format!("price_{}", i), price.to_string());
        }
        
        // Save latest funding rates (up to 3)
        for (i, rate) in self.funding_rates.iter().rev().take(3).enumerate() {
            state.insert(format!("funding_{}", i), rate.to_string());
        }
        
        state
    }
    
    // Load strategy state from a serialized format
    fn load_state(&mut self, state: &HashMap<String, String>) {
        // Load current position
        if let Some(pos) = state.get("current_position") {
            if let Ok(pos_val) = pos.parse::<f64>() {
                self.current_position = pos_val;
            }
        }
        
        // Load parameters (if they exist)
        for (key, value) in state {
            if key.starts_with("param_") {
                let param_name = key.strip_prefix("param_").unwrap();
                self.parameters.insert(param_name.to_string(), value.clone());
            }
        }
    }
}
impl Tra
dingStrategy for EnhancedSmaStrategy {
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
}///
 Run backtest phase of the strategy
async fn run_backtest_phase(symbol: &str) -> Result<(EnhancedSmaStrategy, FundingReport), Box<dyn std::error::Error>> {
    println!("\n📊 Phase 1: Backtesting");
    println!("====================");
    
    // Fetch historical data
    println!("Fetching historical data for {}...", symbol);
    let start_time = chrono::Utc::now() - chrono::Duration::days(30);
    let end_time = chrono::Utc::now();
    
    let data = HyperliquidData::fetch(
        symbol,
        "1h",
        start_time.timestamp() as u64,
        end_time.timestamp() as u64
    ).await?;
    
    println!("Fetched {} data points", data.datetime.len());
    
    // Create strategy
    let strategy = EnhancedSmaStrategy::new(
        symbol,      // Symbol
        12,          // Short period
        26,          // Long period
        0.0001,      // Funding threshold
        0.5,         // Funding weight
        0.1          // Position size (10% of portfolio)
    );
    
    // Create backtest
    println!("Running backtest...");
    let mut backtest = HyperliquidBacktest::new(
        data,
        Box::new(BacktestStrategyAdapter::new(Box::new(strategy.clone()))),
        10000.0,
        HyperliquidCommission::default(),
    )?;
    
    // Run backtest with funding calculations
    backtest.calculate_with_funding()?;
    
    // Get reports
    let report = backtest.funding_report()?;
    let enhanced_report = backtest.enhanced_report()?;
    
    // Print results
    println!("\nBacktest Results:");
    println!("----------------");
    println!("Net profit: ${:.2}", report.net_profit);
    println!("Return: {:.2}%", report.net_profit / 10000.0 * 100.0);
    println!("Sharpe ratio: {:.2}", enhanced_report.sharpe_ratio);
    println!("Max drawdown: {:.2}%", enhanced_report.max_drawdown * 100.0);
    println!("Win rate: {:.2}%", enhanced_report.win_rate * 100.0);
    println!("Trading PnL: ${:.2}", report.net_trading_pnl);
    println!("Funding PnL: ${:.2}", report.net_funding_pnl);
    println!("Total trades: {}", enhanced_report.trade_count);
    
    // Return the strategy and report
    Ok((strategy, report))
}

/// Run paper trading phase of the strategy
async fn run_paper_trading_phase(
    symbol: &str,
    mut strategy: EnhancedSmaStrategy,
    initial_balance: f64
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    println!("\n📝 Phase 2: Paper Trading");
    println!("======================");
    
    // Create slippage configuration
    let slippage_config = SlippageConfig {
        base_slippage_pct: 0.0005,      // 0.05% base slippage
        volume_impact_factor: 0.1,      // Volume impact factor
        volatility_impact_factor: 0.2,  // Volatility impact factor
        random_slippage_max_pct: 0.001, // 0.1% max random component
        simulated_latency_ms: 500,      // 500ms simulated latency
    };
    
    // Create paper trading engine
    println!("Creating paper trading engine...");
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
    
    println!("Connected to real-time data stream");
    println!("Subscribed to {} market data", symbol);
    
    // Start paper trading in a separate task
    let paper_engine_arc = Arc::new(Mutex::new(paper_engine));
    let paper_engine_for_task = paper_engine_arc.clone();
    
    let strategy_box: Box<dyn TradingStrategy> = Box::new(strategy);
    
    let task_handle = tokio::spawn(async move {
        let mut engine = paper_engine_for_task.lock().unwrap();
        if let Err(e) = engine.start_simulation(strategy_box).await {
            eprintln!("Error in paper trading simulation: {}", e);
        }
    });
    
    // Let the simulation run for a while
    println!("Paper trading simulation started. Running for 30 seconds...");
    println!("(In a real application, this would run for days or weeks)");
    
    // Simulate running for 30 seconds
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    
    // Stop the simulation
    {
        let mut engine = paper_engine_arc.lock().unwrap();
        engine.stop_simulation();
        println!("Paper trading simulation stopped");
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
        for (symbol, position) in &positions {
            println!("{}: {} @ ${} (PnL: ${:.2})", 
                symbol, 
                position.size, 
                position.current_price,
                position.total_pnl()
            );
        }
    }
    
    // Save strategy state
    let mut state = strategy.save_state();
    
    // Add position information to state
    if let Some(position) = positions.get(symbol) {
        state.insert("position_size".to_string(), position.size.to_string());
        state.insert("position_entry_price".to_string(), position.entry_price.to_string());
    }
    
    Ok(state)
}/// Run 
live trading phase of the strategy (simulated)
async fn run_live_trading_phase(
    symbol: &str,
    mut strategy: EnhancedSmaStrategy,
    state: HashMap<String, String>,
    initial_balance: f64
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🚀 Phase 3: Live Trading (Simulated)");
    println!("=================================");
    println!("⚠️  This is a simulation and does not execute real trades");
    
    // Load strategy state
    strategy.load_state(&state);
    println!("Loaded strategy state from paper trading phase");
    
    // Create risk configuration with conservative settings for live trading
    let risk_config = RiskConfig {
        max_position_size_pct: 0.05,     // 5% of portfolio (conservative)
        max_daily_loss_pct: 0.01,        // 1% max daily loss (conservative)
        stop_loss_pct: 0.03,             // 3% stop loss (tight)
        take_profit_pct: 0.05,           // 5% take profit
        max_leverage: 2.0,               // 2x max leverage (conservative)
        max_concentration_pct: 0.15,     // 15% max concentration (conservative)
        max_position_correlation: 0.5,   // 50% max correlation
        volatility_sizing_factor: 0.3,   // 30% volatility impact (conservative)
        max_portfolio_volatility_pct: 0.03, // 3% max portfolio volatility (conservative)
        max_drawdown_pct: 0.05,          // 5% max drawdown (conservative)
    };
    
    // Create API configuration
    let api_config = ApiConfig {
        api_key: "your_api_key".to_string(),
        api_secret: "your_api_secret".to_string(),
        endpoint: "https://api.hyperliquid-testnet.xyz".to_string(),
        use_testnet: true, // Always use testnet for examples
        timeout_ms: 5000,
    };
    
    // Create wallet (in a real application, this would be loaded securely)
    println!("Setting up wallet...");
    let private_key = "0000000000000000000000000000000000000000000000000000000000000001";
    let wallet = LocalWallet::from_str(private_key).unwrap();
    
    println!("Creating live trading engine...");
    
    // Create live trading engine (simulated for this example)
    let mut live_engine = SimulatedLiveTradingEngine::new(initial_balance, risk_config)?;
    
    // Initialize position from paper trading if available
    if let Some(position_size) = state.get("position_size") {
        if let Ok(size) = position_size.parse::<f64>() {
            if size != 0.0 {
                let entry_price = state.get("position_entry_price")
                    .and_then(|p| p.parse::<f64>().ok())
                    .unwrap_or(50000.0);
                
                println!("Initializing position from paper trading phase:");
                println!("{}: {} @ ${}", symbol, size, entry_price);
                
                live_engine.initialize_position(
                    symbol,
                    size,
                    entry_price,
                    Utc::now().with_timezone(&FixedOffset::east(0))
                )?;
            }
        }
    }
    
    println!("Starting live trading simulation...");
    
    // Start trading in a separate task
    let live_engine_arc = Arc::new(Mutex::new(live_engine));
    let live_engine_for_task = live_engine_arc.clone();
    
    let strategy_box: Box<dyn TradingStrategy> = Box::new(strategy);
    
    let task_handle = tokio::spawn(async move {
        let mut engine = live_engine_for_task.lock().unwrap();
        if let Err(e) = engine.start_trading(strategy_box).await {
            eprintln!("Error in live trading simulation: {}", e);
        }
    });
    
    // Let the simulation run for a while
    println!("Live trading simulation started. Running for 30 seconds...");
    println!("(In a real application, this would run continuously)");
    
    // Simulate running for 30 seconds
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    
    // Stop trading
    {
        let mut engine = live_engine_arc.lock().unwrap();
        engine.stop_trading().await?;
        println!("Live trading simulation stopped");
        
        // Get final positions
        let positions = engine.get_positions().await?;
        
        println!("\nFinal Positions:");
        if positions.is_empty() {
            println!("No open positions");
        } else {
            for (symbol, position) in positions {
                println!("{}: {} @ ${} (PnL: ${:.2})", 
                    symbol, 
                    position.size, 
                    position.current_price,
                    position.unrealized_pnl
                );
            }
        }
        
        // Get performance metrics
        let metrics = engine.get_performance_metrics().await?;
        
        println!("\nPerformance Metrics:");
        println!("Trading PnL: ${:.2}", metrics.trading_pnl);
        println!("Funding PnL: ${:.2}", metrics.funding_pnl);
        println!("Total PnL: ${:.2}", metrics.total_pnl);
        println!("Total fees: ${:.2}", metrics.total_fees);
    }
    
    // Wait for the task to complete
    let _ = task_handle.await;
    
    Ok(())
}// 
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
        
        let market_data = MarketData::new(
            &data.ticker,
            data.close[index],
            data.low[index],
            data.high[index],
            data.volume[index],
            timestamp,
        );
        
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

// Simulated live trading engine for example purposes
struct SimulatedLiveTradingEngine {
    balance: f64,
    positions: HashMap<String, Position>,
    order_history: Vec<OrderResult>,
    next_order_id: usize,
    risk_config: RiskConfig,
    is_trading: bool,
}

impl SimulatedLiveTradingEngine {
    fn new(initial_balance: f64, risk_config: RiskConfig) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            balance: initial_balance,
            positions: HashMap::new(),
            order_history: Vec::new(),
            next_order_id: 1,
            risk_config,
            is_trading: false,
        })
    }
    
    fn initialize_position(
        &mut self,
        symbol: &str,
        size: f64,
        entry_price: f64,
        timestamp: DateTime<FixedOffset>
    ) -> Result<(), Box<dyn std::error::Error>> {
        let position = Position::new(
            symbol,
            size,
            entry_price,
            entry_price,
            timestamp,
        );
        
        self.positions.insert(symbol.to_string(), position);
        Ok(())
    }
    
    async fn execute_order(&mut self, order: OrderRequest) -> Result<OrderResult, Box<dyn std::error::Error>> {
        // Simulate order execution
        let timestamp = Utc::now().with_timezone(&FixedOffset::east(0));
        let order_id = format!("order_{}", self.next_order_id);
        self.next_order_id += 1;
        
        // Simulate execution price (use requested price or add/subtract small amount)
        let execution_price = match order.price {
            Some(price) => price,
            None => {
                // For market orders, simulate some slippage
                let position = self.positions.get(&order.symbol).cloned();
                match order.side {
                    OrderSide::Buy => {
                        if let Some(pos) = &position {
                            pos.current_price * 1.001 // 0.1% slippage for buys
                        } else {
                            50000.0 // Default price for BTC
                        }
                    },
                    OrderSide::Sell => {
                        if let Some(pos) = &position {
                            pos.current_price * 0.999 // 0.1% slippage for sells
                        } else {
                            50000.0 // Default price for BTC
                        }
                    },
                }
            }
        };
        
        // Calculate fees (0.05% taker fee)
        let fees = execution_price * order.quantity * 0.0005;
        
        // Create order result
        let mut result = OrderResult {
            order_id,
            symbol: order.symbol.clone(),
            side: order.side,
            order_type: order.order_type,
            requested_quantity: order.quantity,
            filled_quantity: order.quantity,
            average_price: Some(execution_price),
            status: OrderStatus::Filled,
            timestamp,
            fees: Some(fees),
            error: None,
            client_order_id: order.client_order_id.clone(),
            metadata: HashMap::new(),
        };
        
        // Update position
        let position_size = match order.side {
            OrderSide::Buy => order.quantity,
            OrderSide::Sell => -order.quantity,
        };
        
        let position = self.positions.entry(order.symbol.clone()).or_insert_with(|| {
            Position::new(
                &order.symbol,
                0.0,
                execution_price,
                execution_price,
                timestamp,
            )
        });
        
        // Update position
        if position.size == 0.0 {
            // New position
            position.size = position_size;
            position.entry_price = execution_price;
        } else if position.size > 0.0 && position_size > 0.0 {
            // Adding to long position
            let new_size = position.size + position_size;
            position.entry_price = (position.entry_price * position.size + execution_price * position_size) / new_size;
            position.size = new_size;
        } else if position.size < 0.0 && position_size < 0.0 {
            // Adding to short position
            let new_size = position.size + position_size;
            position.entry_price = (position.entry_price * position.size + execution_price * position_size) / new_size;
            position.size = new_size;
        } else {
            // Reducing or flipping position
            let new_size = position.size + position_size;
            if new_size.abs() < 0.000001 {
                // Position closed
                position.realized_pnl += position.unrealized_pnl;
                position.size = 0.0;
                position.unrealized_pnl = 0.0;
            } else if new_size * position.size < 0.0 {
                // Position flipped
                let closed_size = position.size;
                let closed_pnl = closed_size * (execution_price - position.entry_price);
                position.realized_pnl += closed_pnl;
                position.size = new_size;
                position.entry_price = execution_price;
                position.unrealized_pnl = 0.0;
            } else {
                // Position reduced
                let closed_size = -position_size;
                let closed_pnl = closed_size * (execution_price - position.entry_price);
                position.realized_pnl += closed_pnl;
                position.size = new_size;
                position.unrealized_pnl = position.size * (execution_price - position.entry_price);
            }
        }
        
        // Update current price
        position.current_price = execution_price;
        position.timestamp = timestamp;
        
        // Update balance (subtract fees)
        self.balance -= fees;
        
        // Add to order history
        self.order_history.push(result.clone());
        
        Ok(result)
    } 
   
    async fn start_trading(&mut self, strategy: Box<dyn TradingStrategy>) -> Result<(), Box<dyn std::error::Error>> {
        self.is_trading = true;
        
        // Simulate market data updates
        let symbol = "BTC"; // Default symbol
        let mut price = 50000.0;
        let mut iteration = 0;
        
        while self.is_trading && iteration < 10 {
            // Simulate price movement
            let price_change = (rand::random::<f64>() - 0.5) * 100.0;
            price += price_change;
            
            // Create market data
            let timestamp = Utc::now().with_timezone(&FixedOffset::east(0));
            let market_data = MarketData::new(
                symbol,
                price,
                price - 10.0,
                price + 10.0,
                1000.0,
                timestamp,
            ).with_funding_rate(
                0.0001 * (rand::random::<f64>() - 0.5),
                timestamp + chrono::Duration::hours(8)
            );
            
            // Process market data with strategy
            let orders = strategy.on_market_data(&market_data)
                .map_err(|e| format!("Strategy error: {}", e))?;
            
            // Execute orders
            for order in orders {
                let result = self.execute_order(order).await?;
                
                if result.status == OrderStatus::Filled || result.status == OrderStatus::PartiallyFilled {
                    // Notify strategy of fill
                    strategy.on_order_fill(&result)
                        .map_err(|e| format!("Strategy error: {}", e))?;
                    
                    println!("Order executed: {} {} {} at ${} with size {}",
                        result.symbol,
                        result.side,
                        result.order_type,
                        result.average_price.unwrap_or(0.0),
                        result.filled_quantity
                    );
                }
            }
            
            // Update positions with current price
            for (_, position) in &mut self.positions {
                position.current_price = price;
                position.unrealized_pnl = position.size * (price - position.entry_price);
            }
            
            // Print current status
            let account_value = self.get_account_value().await?;
            println!("Iteration {}: Account value: ${:.2}", iteration + 1, account_value);
            
            // Simulate delay between updates
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            
            iteration += 1;
        }
        
        Ok(())
    }
    
    async fn stop_trading(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.is_trading = false;
        Ok(())
    }
    
    async fn get_positions(&self) -> Result<HashMap<String, Position>, Box<dyn std::error::Error>> {
        Ok(self.positions.clone())
    }
    
    async fn get_account_value(&self) -> Result<f64, Box<dyn std::error::Error>> {
        let mut total = self.balance;
        
        // Add unrealized PnL from positions
        for (_, position) in &self.positions {
            total += position.unrealized_pnl;
        }
        
        Ok(total)
    }
    
    async fn get_performance_metrics(&self) -> Result<PerformanceMetrics, Box<dyn std::error::Error>> {
        let mut trading_pnl = 0.0;
        let mut funding_pnl = 0.0;
        
        // Sum up realized PnL and funding PnL
        for (_, position) in &self.positions {
            trading_pnl += position.realized_pnl + position.unrealized_pnl;
            funding_pnl += position.funding_pnl;
        }
        
        Ok(PerformanceMetrics {
            trading_pnl,
            funding_pnl,
            total_pnl: trading_pnl + funding_pnl,
            total_fees: self.order_history.iter()
                .filter_map(|order| order.fees)
                .sum(),
        })
    }
}

// Simple performance metrics structure
struct PerformanceMetrics {
    trading_pnl: f64,
    funding_pnl: f64,
    total_pnl: f64,
    total_fees: f64,
}

// Clone implementation for EnhancedSmaStrategy
impl Clone for EnhancedSmaStrategy {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            symbol: self.symbol.clone(),
            short_period: self.short_period,
            long_period: self.long_period,
            prices: self.prices.clone(),
            funding_rates: self.funding_rates.clone(),
            funding_threshold: self.funding_threshold,
            funding_weight: self.funding_weight,
            position_size: self.position_size,
            current_position: self.current_position,
            signals: self.signals.clone(),
            parameters: self.parameters.clone(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    init_logger();
    
    println!("Strategy Migration Example: Backtest → Paper → Live");
    println!("=================================================");
    println!("This example demonstrates the workflow of developing and");
    println!("migrating a trading strategy through three phases:");
    println!("1. Backtesting with historical data");
    println!("2. Paper trading with real-time data");
    println!("3. Live trading with real execution (simulated)");
    
    // Define trading symbol
    let symbol = "BTC";
    let initial_balance = 10000.0;
    
    // Phase 1: Backtesting
    let (strategy, backtest_report) = run_backtest_phase(symbol).await?;
    
    // Phase 2: Paper Trading
    let strategy_state = run_paper_trading_phase(symbol, strategy.clone(), initial_balance).await?;
    
    // Phase 3: Live Trading (Simulated)
    run_live_trading_phase(symbol, strategy, strategy_state, initial_balance).await?;
    
    println!("\nStrategy Migration Example Completed Successfully!");
    println!("In a real-world scenario, each phase would run for much longer");
    println!("periods, with careful analysis between phases.");
    
    Ok(())
}