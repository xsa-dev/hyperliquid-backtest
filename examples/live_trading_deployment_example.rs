use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, FixedOffset, Utc};
use ethers::signers::LocalWallet;

use hyperliquid_backtest::prelude::*;
use hyperliquid_backtest::live_trading::{
    LiveTradingEngine, LiveTradingError, AlertLevel, AlertMessage, 
    RetryPolicy, SafetyCircuitBreakerConfig
};
use hyperliquid_backtest::trading_mode::{ApiConfig, RiskConfig, TradingConfig};
use hyperliquid_backtest::unified_data::{
    Position, OrderRequest, OrderResult, MarketData, 
    OrderSide, OrderType, TimeInForce, OrderStatus,
    TradingStrategy, Signal, SignalDirection, FundingPayment
};
use hyperliquid_backtest::logging::init_logger;

/// A funding arbitrage strategy that takes positions based on funding rate opportunities
struct FundingArbitrageStrategy {
    name: String,
    symbols: Vec<String>,
    funding_threshold: f64,
    position_size_base: f64,
    max_positions: usize,
    positions: HashMap<String, f64>,
    signals: HashMap<String, Signal>,
    funding_rates: HashMap<String, f64>,
    next_funding_times: HashMap<String, DateTime<FixedOffset>>,
    last_position_change: HashMap<String, DateTime<FixedOffset>>,
    min_hold_hours: i64,
}

impl FundingArbitrageStrategy {
    fn new(
        symbols: Vec<String>,
        funding_threshold: f64,
        position_size_base: f64,
        max_positions: usize,
        min_hold_hours: i64,
    ) -> Self {
        Self {
            name: "Funding Arbitrage Strategy".to_string(),
            symbols,
            funding_threshold,
            position_size_base,
            max_positions,
            positions: HashMap::new(),
            signals: HashMap::new(),
            funding_rates: HashMap::new(),
            next_funding_times: HashMap::new(),
            last_position_change: HashMap::new(),
            min_hold_hours,
        }
    }
    
    fn should_take_position(&self, symbol: &str, funding_rate: f64) -> Option<OrderSide> {
        // Check if funding rate exceeds threshold
        if funding_rate.abs() < self.funding_threshold {
            return None;
        }
        
        // Check if we already have a position in this symbol
        if let Some(position_size) = self.positions.get(symbol) {
            // If we have a position, check if it's in the right direction
            if (funding_rate > 0.0 && *position_size < 0.0) || 
               (funding_rate < 0.0 && *position_size > 0.0) {
                // Position is already in the correct direction
                return None;
            }
            
            // Check if we've held the position for minimum time
            if let Some(last_change) = self.last_position_change.get(symbol) {
                let now = Utc::now().with_timezone(&FixedOffset::east(0));
                let hours_since_change = (now - *last_change).num_hours();
                
                if hours_since_change < self.min_hold_hours {
                    // Haven't held position for minimum time
                    return None;
                }
            }
        }
        
        // Determine position side based on funding rate
        // For funding arbitrage:
        // - When funding rate is positive, go short (collect funding)
        // - When funding rate is negative, go long (collect funding)
        if funding_rate > 0.0 {
            Some(OrderSide::Sell) // Short position
        } else {
            Some(OrderSide::Buy) // Long position
        }
    }
    
    fn calculate_position_size(&self, symbol: &str, funding_rate: f64) -> f64 {
        // Base position size
        let mut size = self.position_size_base;
        
        // Scale position size based on funding rate magnitude
        // Higher funding rate = larger position (up to 2x base size)
        let scale_factor = 1.0 + (funding_rate.abs() / self.funding_threshold - 1.0).min(1.0);
        size *= scale_factor;
        
        size
    }
}

impl TradingStrategy for FundingArbitrageStrategy {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
        let symbol = &data.symbol;
        
        // Skip if not in our watchlist
        if !self.symbols.contains(symbol) {
            return Ok(Vec::new());
        }
        
        // Update funding rate if available
        if let Some(funding_rate) = data.funding_rate {
            self.funding_rates.insert(symbol.clone(), funding_rate);
            
            // Update next funding time if available
            if let Some(next_funding) = data.next_funding_time {
                self.next_funding_times.insert(symbol.clone(), next_funding);
            }
        } else {
            // Skip if no funding rate available
            return Ok(Vec::new());
        }
        
        let now = Utc::now().with_timezone(&FixedOffset::east(0));
        let funding_rate = *self.funding_rates.get(symbol).unwrap();
        
        // Check if we should take a position
        let mut orders = Vec::new();
        
        if let Some(side) = self.should_take_position(symbol, funding_rate) {
            // Calculate position size
            let position_size = self.calculate_position_size(symbol, funding_rate);
            
            // Check if we already have a position that needs to be closed
            if let Some(current_size) = self.positions.get(symbol) {
                if *current_size != 0.0 {
                    // Close existing position
                    let close_side = if *current_size > 0.0 {
                        OrderSide::Sell
                    } else {
                        OrderSide::Buy
                    };
                    
                    orders.push(OrderRequest {
                        symbol: symbol.clone(),
                        side: close_side,
                        order_type: OrderType::Market,
                        quantity: current_size.abs(),
                        price: None,
                        reduce_only: true,
                        time_in_force: TimeInForce::ImmediateOrCancel,
                        client_order_id: Some(format!("close_{}_{}", symbol, now.timestamp())),
                        metadata: HashMap::new(),
                    });
                }
            }
            
            // Open new position
            orders.push(OrderRequest {
                symbol: symbol.clone(),
                side,
                order_type: OrderType::Market,
                quantity: position_size,
                price: None,
                reduce_only: false,
                time_in_force: TimeInForce::ImmediateOrCancel,
                client_order_id: Some(format!("open_{}_{}", symbol, now.timestamp())),
                metadata: HashMap::new(),
            });
            
            // Update signal
            let direction = match side {
                OrderSide::Buy => SignalDirection::Buy,
                OrderSide::Sell => SignalDirection::Sell,
            };
            
            self.signals.insert(
                symbol.clone(),
                Signal {
                    symbol: symbol.clone(),
                    direction,
                    strength: (funding_rate.abs() / self.funding_threshold).min(1.0),
                    timestamp: now,
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("funding_rate".to_string(), funding_rate.to_string());
                        metadata.insert("position_size".to_string(), position_size.to_string());
                        metadata
                    },
                }
            );
        }
        
        Ok(orders)
    }
    
    fn on_order_fill(&mut self, fill: &OrderResult) -> Result<(), String> {
        let symbol = &fill.symbol;
        
        // Update position tracking
        let current_position = self.positions.entry(symbol.clone()).or_insert(0.0);
        
        match fill.side {
            OrderSide::Buy => {
                *current_position += fill.filled_quantity;
            },
            OrderSide::Sell => {
                *current_position -= fill.filled_quantity;
            },
        }
        
        // Update last position change time
        self.last_position_change.insert(
            symbol.clone(),
            Utc::now().with_timezone(&FixedOffset::east(0))
        );
        
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

/// Performs pre-deployment safety checks
async fn perform_safety_checks(engine: &LiveTradingEngine) -> Result<bool, LiveTradingError> {
    println!("\n🔍 Performing Pre-Deployment Safety Checks");
    println!("----------------------------------------");
    
    // Check 1: Verify connection to exchange
    println!("1. Verifying connection to exchange...");
    if !engine.is_connected() {
        println!("❌ Not connected to exchange");
        return Ok(false);
    }
    println!("✅ Connected to exchange");
    
    // Check 2: Verify account balance
    println!("2. Verifying account balance...");
    let account_info = engine.get_account_info().await?;
    if account_info.available_balance < 100.0 {
        println!("❌ Insufficient balance: ${:.2}", account_info.available_balance);
        return Ok(false);
    }
    println!("✅ Account balance: ${:.2}", account_info.available_balance);
    
    // Check 3: Verify market data access
    println!("3. Verifying market data access...");
    let btc_price = engine.get_current_price("BTC").await?;
    if btc_price <= 0.0 {
        println!("❌ Unable to fetch market data");
        return Ok(false);
    }
    println!("✅ Market data accessible (BTC price: ${:.2})", btc_price);
    
    // Check 4: Verify order placement capability
    println!("4. Verifying order placement capability...");
    let can_place_orders = engine.can_place_orders().await?;
    if !can_place_orders {
        println!("❌ Cannot place orders");
        return Ok(false);
    }
    println!("✅ Order placement capability verified");
    
    // Check 5: Verify risk limits
    println!("5. Verifying risk limits...");
    let risk_limits = engine.get_risk_limits().await?;
    println!("✅ Risk limits verified:");
    println!("   - Max position size: ${:.2}", risk_limits.max_position_size);
    println!("   - Max leverage: {:.1}x", risk_limits.max_leverage);
    println!("   - Max daily loss: ${:.2}", risk_limits.max_daily_loss);
    
    // Check 6: Verify emergency stop functionality
    println!("6. Verifying emergency stop functionality...");
    if !engine.test_emergency_stop().await? {
        println!("❌ Emergency stop functionality not working");
        return Ok(false);
    }
    println!("✅ Emergency stop functionality verified");
    
    // All checks passed
    println!("\n✅ All safety checks passed");
    Ok(true)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    init_logger();
    
    println!("Hyperliquid Live Trading Deployment Example");
    println!("=========================================");
    println!("⚠️  This example simulates a live trading deployment");
    println!("⚠️  No actual trades will be executed");
    
    // Step 1: Create trading configuration
    println!("\n1. Creating Trading Configuration");
    println!("--------------------------------");
    
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
    
    // Create trading configuration
    let trading_config = TradingConfig::new(10000.0)  // $10,000 initial balance
        .with_risk_config(risk_config)
        .with_api_config(api_config)
        .with_parameter("enable_alerts", "true")
        .with_parameter("max_open_orders", "3");
    
    println!("✅ Trading configuration created with conservative risk settings");
    
    // Step 2: Create wallet (in a real application, this would be loaded securely)
    println!("\n2. Setting Up Wallet");
    println!("-------------------");
    
    // WARNING: This is a dummy private key for demonstration only
    let private_key = "0000000000000000000000000000000000000000000000000000000000000001";
    let wallet = LocalWallet::from_str(private_key).unwrap();
    
    println!("✅ Wallet created");
    println!("📝 Wallet address: {}", wallet.address());
    
    // Step 3: Create custom safety configurations
    println!("\n3. Configuring Safety Mechanisms");
    println!("-------------------------------");
    
    // Create custom retry policy
    let retry_policy = RetryPolicy {
        max_attempts: 3,
        initial_delay_ms: 500,
        backoff_factor: 2.0,
        max_delay_ms: 5000,
    };
    
    // Create custom safety circuit breaker configuration
    let safety_config = SafetyCircuitBreakerConfig {
        max_consecutive_failed_orders: 3,
        max_order_failure_rate: 0.3,
        order_failure_rate_window: 10,
        max_position_drawdown_pct: 0.10,
        max_account_drawdown_pct: 0.05,
        max_price_deviation_pct: 0.03,
        price_deviation_window_sec: 60,
        max_critical_alerts: 2,
        critical_alerts_window: 10,
    };
    
    println!("✅ Safety mechanisms configured:");
    println!("   - Retry policy: {} attempts with backoff", retry_policy.max_attempts);
    println!("   - Circuit breaker: {:.1}% max account drawdown", safety_config.max_account_drawdown_pct * 100.0);
    println!("   - Price deviation protection: {:.1}%", safety_config.max_price_deviation_pct * 100.0);
    
    // Step 4: Create live trading engine
    println!("\n4. Creating Live Trading Engine");
    println!("------------------------------");
    
    // Create live trading engine
    let mut engine = LiveTradingEngine::new(
        wallet, 
        trading_config.risk_config.unwrap(), 
        trading_config.api_config.unwrap()
    ).await?;
    
    // Configure safety mechanisms
    engine.set_retry_policy(retry_policy);
    engine.set_safety_circuit_breaker_config(safety_config);
    engine.set_detailed_logging(true);
    
    println!("✅ Live trading engine created");
    
    // Step 5: Initialize safety mechanisms
    println!("\n5. Initializing Safety Mechanisms");
    println!("--------------------------------");
    
    // Initialize safety mechanisms
    engine.init_safety_mechanisms().await?;
    
    println!("✅ Safety mechanisms initialized");
    
    // Step 6: Connect to exchange
    println!("\n6. Connecting to Exchange");
    println!("------------------------");
    
    // Connect to exchange
    engine.connect().await?;
    
    println!("✅ Connected to exchange");
    
    // Step 7: Perform pre-deployment safety checks
    let checks_passed = perform_safety_checks(&engine).await?;
    
    if !checks_passed {
        println!("\n❌ Safety checks failed. Aborting deployment.");
        engine.disconnect().await?;
        return Ok(());
    }
    
    // Step 8: Create trading strategy
    println!("\n8. Creating Trading Strategy");
    println!("---------------------------");
    
    // Create funding arbitrage strategy
    let strategy = Box::new(FundingArbitrageStrategy::new(
        vec!["BTC".to_string(), "ETH".to_string()], // Symbols to trade
        0.0002,                                     // Funding threshold (0.02% per 8h)
        0.1,                                        // Base position size (10% of portfolio)
        2,                                          // Max positions
        24,                                         // Minimum hold hours
    ));
    
    println!("✅ Created Funding Arbitrage Strategy");
    println!("   - Trading BTC, ETH");
    println!("   - Funding threshold: 0.02% per 8h");
    println!("   - Base position size: 10% of portfolio");
    println!("   - Minimum hold period: 24 hours");
    
    // Step 9: Register alert handlers
    println!("\n9. Registering Alert Handlers");
    println!("----------------------------");
    
    // Register alert handler
    engine.register_alert_handler(|alert: &AlertMessage| {
        match alert.level {
            AlertLevel::Info => println!("ℹ️ INFO: {}", alert.message),
            AlertLevel::Warning => println!("⚠️ WARNING: {}", alert.message),
            AlertLevel::Critical => println!("🚨 CRITICAL: {}", alert.message),
        }
        
        // Return true to continue trading, false to trigger emergency stop
        alert.level != AlertLevel::Critical
    });
    
    println!("✅ Alert handlers registered");
    
    // Step 10: Start trading
    println!("\n10. Starting Live Trading");
    println!("------------------------");
    println!("⚠️  In a real deployment, this would execute actual trades");
    println!("⚠️  This example will simulate trading for 30 seconds");
    
    // Start trading in a separate task
    let engine_arc = Arc::new(Mutex::new(engine));
    let engine_for_task = engine_arc.clone();
    
    let task_handle = tokio::spawn(async move {
        let mut engine = engine_for_task.lock().unwrap();
        if let Err(e) = engine.start_trading(strategy).await {
            eprintln!("Error in live trading: {}", e);
            
            // If emergency stop was triggered, show details
            if engine.is_emergency_stop_active() {
                eprintln!("⚠️ Emergency stop was triggered");
                
                // Show positions at time of emergency stop
                if let Ok(positions) = engine.get_positions() {
                    eprintln!("📊 Positions at emergency stop:");
                    for (symbol, position) in positions {
                        eprintln!("  {}: {} @ {:.2} (PnL: {:.2})", 
                                symbol, position.size, position.current_price, position.unrealized_pnl);
                    }
                }
            }
        }
    });
    
    // Let the simulation run for a while
    println!("Live trading started. Running for 30 seconds...");
    println!("(In a real application, this would run continuously)");
    
    // Simulate running for 30 seconds
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    
    // Step 11: Graceful shutdown
    println!("\n11. Performing Graceful Shutdown");
    println!("-------------------------------");
    
    // Stop trading
    {
        let mut engine = engine_arc.lock().unwrap();
        engine.stop_trading().await?;
        println!("✅ Trading stopped");
        
        // Get final positions
        let positions = engine.get_positions();
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
        
        // Disconnect from exchange
        engine.disconnect().await?;
        println!("✅ Disconnected from exchange");
    }
    
    // Wait for the task to complete
    let _ = task_handle.await;
    
    println!("\nExample completed successfully!");
    println!("In a real deployment, the trading engine would continue running");
    println!("and would require proper monitoring and maintenance procedures.");
    
    Ok(())
}