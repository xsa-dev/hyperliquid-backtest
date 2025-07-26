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
use hyperliquid_backtest::trading_mode::{ApiConfig, RiskConfig};
use hyperliquid_backtest::unified_data::{
    Position, OrderRequest, OrderResult, MarketData, 
    OrderSide, OrderType, TimeInForce, OrderStatus,
    TradingStrategy, Signal
};
use hyperliquid_backtest::logging::init_logger;

// Simple moving average crossover strategy
struct SmaCrossStrategy {
    symbol: String,
    short_period: usize,
    long_period: usize,
    short_ma: Vec<f64>,
    long_ma: Vec<f64>,
    prices: Vec<f64>,
    position_size: f64,
    current_position: f64,
}

impl SmaCrossStrategy {
    fn new(symbol: &str, short_period: usize, long_period: usize, position_size: f64) -> Self {
        Self {
            symbol: symbol.to_string(),
            short_period,
            long_period,
            short_ma: Vec::new(),
            long_ma: Vec::new(),
            prices: Vec::new(),
            position_size,
            current_position: 0.0,
        }
    }
    
    fn calculate_sma(&self, period: usize) -> Option<f64> {
        if self.prices.len() < period {
            return None;
        }
        
        let sum: f64 = self.prices.iter().rev().take(period).sum();
        Some(sum / period as f64)
    }
    
    fn update_indicators(&mut self, price: f64) {
        self.prices.push(price);
        
        if let Some(short_ma) = self.calculate_sma(self.short_period) {
            self.short_ma.push(short_ma);
        }
        
        if let Some(long_ma) = self.calculate_sma(self.long_period) {
            self.long_ma.push(long_ma);
        }
        
        // Keep the price history manageable
        if self.prices.len() > self.long_period * 2 {
            self.prices.remove(0);
        }
        
        if self.short_ma.len() > 10 {
            self.short_ma.remove(0);
        }
        
        if self.long_ma.len() > 10 {
            self.long_ma.remove(0);
        }
    }
    
    fn get_signal(&self) -> Option<OrderSide> {
        if self.short_ma.len() < 2 || self.long_ma.len() < 2 {
            return None;
        }
        
        let short_ma_current = self.short_ma.last().unwrap();
        let short_ma_prev = self.short_ma.get(self.short_ma.len() - 2).unwrap();
        
        let long_ma_current = self.long_ma.last().unwrap();
        let long_ma_prev = self.long_ma.get(self.long_ma.len() - 2).unwrap();
        
        // Crossover: short MA crosses above long MA
        if short_ma_prev <= long_ma_prev && short_ma_current > long_ma_current {
            return Some(OrderSide::Buy);
        }
        
        // Crossunder: short MA crosses below long MA
        if short_ma_prev >= long_ma_prev && short_ma_current < long_ma_current {
            return Some(OrderSide::Sell);
        }
        
        None
    }
}

impl TradingStrategy for SmaCrossStrategy {
    fn name(&self) -> &str {
        "SMA Crossover Strategy"
    }
    
    fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
        if data.symbol != self.symbol {
            return Ok(Vec::new());
        }
        
        // Update indicators with new price
        self.update_indicators(data.price);
        
        // Get trading signal
        let signal = self.get_signal();
        
        // Generate orders based on signal
        let mut orders = Vec::new();
        
        if let Some(side) = signal {
            match side {
                OrderSide::Buy => {
                    if self.current_position <= 0.0 {
                        // Close any short position
                        if self.current_position < 0.0 {
                            orders.push(OrderRequest {
                                symbol: self.symbol.clone(),
                                side: OrderSide::Buy,
                                order_type: OrderType::Market,
                                quantity: self.current_position.abs(),
                                price: None,
                                reduce_only: true,
                                time_in_force: TimeInForce::ImmediateOrCancel,
                            });
                        }
                        
                        // Open long position
                        orders.push(OrderRequest {
                            symbol: self.symbol.clone(),
                            side: OrderSide::Buy,
                            order_type: OrderType::Market,
                            quantity: self.position_size,
                            price: None,
                            reduce_only: false,
                            time_in_force: TimeInForce::ImmediateOrCancel,
                        });
                        
                        self.current_position = self.position_size;
                    }
                },
                OrderSide::Sell => {
                    if self.current_position >= 0.0 {
                        // Close any long position
                        if self.current_position > 0.0 {
                            orders.push(OrderRequest {
                                symbol: self.symbol.clone(),
                                side: OrderSide::Sell,
                                order_type: OrderType::Market,
                                quantity: self.current_position,
                                price: None,
                                reduce_only: true,
                                time_in_force: TimeInForce::ImmediateOrCancel,
                            });
                        }
                        
                        // Open short position
                        orders.push(OrderRequest {
                            symbol: self.symbol.clone(),
                            side: OrderSide::Sell,
                            order_type: OrderType::Market,
                            quantity: self.position_size,
                            price: None,
                            reduce_only: false,
                            time_in_force: TimeInForce::ImmediateOrCancel,
                        });
                        
                        self.current_position = -self.position_size;
                    }
                },
            }
        }
        
        Ok(orders)
    }
    
    fn on_order_fill(&mut self, fill: &crate::unified_data::OrderFill) -> Result<(), String> {
        // Update position based on fill
        if fill.symbol == self.symbol {
            match fill.side {
                OrderSide::Buy => {
                    self.current_position += fill.quantity;
                },
                OrderSide::Sell => {
                    self.current_position -= fill.quantity;
                },
            }
        }
        
        Ok(())
    }
    
    fn on_funding_payment(&mut self, _payment: &crate::unified_data::FundingPayment) -> Result<(), String> {
        // Not handling funding payments in this example
        Ok(())
    }
    
    fn get_current_signals(&self) -> HashMap<String, Signal> {
        let mut signals = HashMap::new();
        
        if let Some(signal_side) = self.get_signal() {
            let direction = match signal_side {
                OrderSide::Buy => SignalDirection::Long,
                OrderSide::Sell => SignalDirection::Short,
            };
            
            signals.insert(
                self.symbol.clone(),
                Signal {
                    symbol: self.symbol.clone(),
                    direction,
                    strength: 1.0,
                    timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
                }
            );
        }
        
        signals
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    init_logger();
    
    println!("🛡️ Live Trading Safety Example");
    println!("==============================");
    
    // Create wallet (in a real application, this would be loaded securely)
    // WARNING: This is a dummy private key for demonstration only
    let private_key = "0000000000000000000000000000000000000000000000000000000000000001";
    let wallet = LocalWallet::from_str(private_key).unwrap();
    
    println!("📝 Wallet address: {}", wallet.address());
    
    // Create API configuration
    let api_config = ApiConfig {
        api_key: "your_api_key".to_string(),
        api_secret: "your_api_secret".to_string(),
        endpoint: "https://api.hyperliquid-testnet.xyz".to_string(),
        use_testnet: true, // Always use testnet for examples
        timeout_ms: 5000,
    };
    
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
        max_order_failure_rate: 0.5,
        order_failure_rate_window: 10,
        max_position_drawdown_pct: 0.15,
        max_account_drawdown_pct: 0.10,
        max_price_deviation_pct: 0.05,
        price_deviation_window_sec: 60,
        max_critical_alerts: 3,
        critical_alerts_window: 10,
    };
    
    println!("⚙️ Creating live trading engine...");
    
    // Create live trading engine
    let mut engine = LiveTradingEngine::new(wallet, risk_config, api_config).await?;
    
    // Configure safety mechanisms
    engine.set_retry_policy(retry_policy);
    engine.set_safety_circuit_breaker_config(safety_config);
    engine.set_detailed_logging(true);
    
    println!("🛡️ Initializing safety mechanisms...");
    
    // Initialize safety mechanisms
    engine.init_safety_mechanisms().await?;
    
    println!("🔌 Connecting to exchange...");
    
    // Connect to exchange
    engine.connect().await?;
    
    println!("✅ Connected to exchange");
    
    // Create trading strategy
    let strategy = Box::new(SmaCrossStrategy::new("BTC", 10, 20, 0.01));
    
    println!("🚀 Starting trading with strategy: {}", strategy.name());
    
    // Start trading
    match engine.start_trading(strategy).await {
        Ok(_) => {
            println!("✅ Trading completed successfully");
        },
        Err(e) => {
            println!("❌ Trading error: {}", e);
            
            // If emergency stop was triggered, show details
            if engine.is_emergency_stop_active() {
                println!("⚠️ Emergency stop was triggered");
                
                // Show positions at time of emergency stop
                println!("📊 Positions at emergency stop:");
                for (symbol, position) in engine.get_positions() {
                    println!("  {}: {} @ {:.2} (PnL: {:.2})", 
                            symbol, position.size, position.current_price, position.unrealized_pnl);
                }
            }
        }
    }
    
    // Disconnect from exchange
    println!("🔌 Disconnecting from exchange...");
    engine.disconnect().await?;
    
    println!("✅ Disconnected from exchange");
    
    Ok(())
}