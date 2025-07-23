use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use chrono::{DateTime, FixedOffset, Utc};
use ethers::signers::LocalWallet;
use tokio::test;

use crate::live_trading::{
    LiveTradingEngine, LiveTradingError, AlertLevel, AlertMessage, 
    RetryPolicy, SafetyCircuitBreakerConfig
};
use crate::trading_mode::{ApiConfig, RiskConfig};
use crate::unified_data::{
    Position, OrderRequest, OrderResult, MarketData, 
    OrderSide, OrderType, TimeInForce, OrderStatus,
    TradingStrategy, Signal
};
use crate::real_time_data_stream::RealTimeDataStream;

// Mock implementation of TradingStrategy for testing
struct MockStrategy {
    name: String,
    signals: HashMap<String, Signal>,
    should_generate_orders: bool,
}

impl MockStrategy {
    fn new(name: &str, should_generate_orders: bool) -> Self {
        Self {
            name: name.to_string(),
            signals: HashMap::new(),
            should_generate_orders,
        }
    }
}

impl TradingStrategy for MockStrategy {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
        // Store signal
        let signal = Signal {
            symbol: data.symbol.clone(),
            direction: if data.price > data.mid_price() {
                crate::unified_data::SignalDirection::Buy
            } else {
                crate::unified_data::SignalDirection::Sell
            },
            strength: 0.8,
            timestamp: data.timestamp,
            metadata: HashMap::new(),
        };
        
        self.signals.insert(data.symbol.clone(), signal);
        
        // Generate orders based on flag
        if self.should_generate_orders {
            if data.price > data.mid_price() {
                Ok(vec![OrderRequest::market(&data.symbol, OrderSide::Buy, 0.001)])
            } else {
                Ok(vec![OrderRequest::market(&data.symbol, OrderSide::Sell, 0.001)])
            }
        } else {
            Ok(vec![])
        }
    }
    
    fn on_order_fill(&mut self, _fill: &crate::unified_data::OrderFill) -> Result<(), String> {
        Ok(())
    }
    
    fn on_funding_payment(&mut self, _payment: &crate::unified_data::FundingPayment) -> Result<(), String> {
        Ok(())
    }
    
    fn get_current_signals(&self) -> HashMap<String, Signal> {
        self.signals.clone()
    }
}

// Helper function to create a test wallet
fn create_test_wallet() -> LocalWallet {
    // This is a test private key, never use in production
    let private_key = "0000000000000000000000000000000000000000000000000000000000000001";
    LocalWallet::from_str(private_key).unwrap()
}

// Helper function to create a test API config for testnet
fn create_test_api_config() -> ApiConfig {
    ApiConfig {
        api_key: "test_key".to_string(),
        api_secret: "test_secret".to_string(),
        endpoint: "https://api.hyperliquid-testnet.xyz".to_string(),
        use_testnet: true,
        timeout_ms: 5000,
    }
}

// Helper function to create a test risk config
fn create_test_risk_config() -> RiskConfig {
    RiskConfig {
        max_position_size_pct: 0.01,      // 1% of portfolio
        max_daily_loss_pct: 0.02,         // 2% max daily loss
        stop_loss_pct: 0.05,              // 5% stop loss
        take_profit_pct: 0.1,             // 10% take profit
        max_leverage: 2.0,                // 2x max leverage
        max_concentration_pct: 0.1,       // 10% max concentration
        max_position_correlation: 0.5,    // 0.5 maximum correlation
        max_portfolio_volatility_pct: 0.1, // 10% maximum portfolio volatility
        volatility_sizing_factor: 0.3,    // 30% volatility-based position sizing
        max_drawdown_pct: 0.1,            // 10% maximum drawdown
    }
}

// Helper function to create a test order request
fn create_test_order(symbol: &str, side: OrderSide, quantity: f64, price: Option<f64>) -> OrderRequest {
    OrderRequest {
        symbol: symbol.to_string(),
        side,
        order_type: if price.is_some() { OrderType::Limit } else { OrderType::Market },
        quantity,
        price,
        reduce_only: false,
        time_in_force: TimeInForce::GoodTillCancel,
    }
}

// Note: These tests are marked with #[ignore] because they require actual API access
// To run these tests, you need to provide valid API credentials and run with:
// cargo test -- --ignored

#[tokio::test]
#[ignore]
async fn test_live_trading_engine_initialization() {
    let wallet = create_test_wallet();
    let risk_config = create_test_risk_config();
    let api_config = create_test_api_config();
    
    let engine = LiveTradingEngine::new(wallet, risk_config, api_config).await;
    assert!(engine.is_ok());
    
    let engine = engine.unwrap();
    assert!(!engine.is_connected());
    assert_eq!(engine.get_account_balance(), 0.0); // Should be updated after connect
}

#[tokio::test]
#[ignore]
async fn test_live_trading_connect_disconnect() {
    let wallet = create_test_wallet();
    let risk_config = create_test_risk_config();
    let api_config = create_test_api_config();
    
    let mut engine = LiveTradingEngine::new(wallet, risk_config, api_config).await.unwrap();
    
    // Connect
    let result = engine.connect().await;
    assert!(result.is_ok());
    assert!(engine.is_connected());
    
    // Disconnect
    let result = engine.disconnect().await;
    assert!(result.is_ok());
    assert!(!engine.is_connected());
}

#[tokio::test]
#[ignore]
async fn test_live_trading_market_data() {
    let wallet = create_test_wallet();
    let risk_config = create_test_risk_config();
    let api_config = create_test_api_config();
    
    let mut engine = LiveTradingEngine::new(wallet, risk_config, api_config).await.unwrap();
    
    // Connect
    engine.connect().await.unwrap();
    
    // Subscribe to market data
    if let Some(data_stream) = &engine.real_time_data {
        let mut stream = data_stream.lock().unwrap();
        let result = stream.subscribe_to_ticker("BTC").await;
        assert!(result.is_ok());
    }
    
    // Wait for some market data
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Check if we received market data
    let market_data = engine.get_market_data("BTC");
    assert!(market_data.is_ok());
    
    let data = market_data.unwrap();
    assert_eq!(data.symbol, "BTC");
    assert!(data.price > 0.0);
    
    // Disconnect
    engine.disconnect().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_live_trading_order_execution() {
    let wallet = create_test_wallet();
    let risk_config = create_test_risk_config();
    let api_config = create_test_api_config();
    
    let mut engine = LiveTradingEngine::new(wallet, risk_config, api_config).await.unwrap();
    
    // Connect
    engine.connect().await.unwrap();
    
    // Subscribe to market data
    if let Some(data_stream) = &engine.real_time_data {
        let mut stream = data_stream.lock().unwrap();
        let result = stream.subscribe_to_ticker("BTC").await;
        assert!(result.is_ok());
    }
    
    // Wait for market data
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Create a very small market buy order (to minimize cost of test)
    let order = create_test_order("BTC", OrderSide::Buy, 0.0001, None);
    
    // Execute order
    let result = engine.execute_order(order).await;
    assert!(result.is_ok());
    
    let order_result = result.unwrap();
    assert_eq!(order_result.status, OrderStatus::Filled);
    assert_eq!(order_result.filled_quantity, 0.0001);
    assert!(order_result.average_price.is_some());
    
    // Check position was created
    let positions = engine.get_positions();
    assert!(positions.contains_key("BTC"));
    
    let btc_position = positions.get("BTC").unwrap();
    assert_eq!(btc_position.size, 0.0001);
    
    // Create a market sell order to close position
    let order = create_test_order("BTC", OrderSide::Sell, 0.0001, None);
    
    // Execute order
    let result = engine.execute_order(order).await;
    assert!(result.is_ok());
    
    // Disconnect
    engine.disconnect().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_live_trading_limit_orders() {
    let wallet = create_test_wallet();
    let risk_config = create_test_risk_config();
    let api_config = create_test_api_config();
    
    let mut engine = LiveTradingEngine::new(wallet, risk_config, api_config).await.unwrap();
    
    // Connect
    engine.connect().await.unwrap();
    
    // Subscribe to market data
    if let Some(data_stream) = &engine.real_time_data {
        let mut stream = data_stream.lock().unwrap();
        let result = stream.subscribe_to_ticker("BTC").await;
        assert!(result.is_ok());
    }
    
    // Wait for market data
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Get current price
    let market_data = engine.get_market_data("BTC").unwrap();
    let current_price = market_data.price;
    
    // Create a limit buy order 5% below current price
    let limit_price = current_price * 0.95;
    let order = create_test_order("BTC", OrderSide::Buy, 0.0001, Some(limit_price));
    
    // Submit order
    let result = engine.execute_order(order).await;
    assert!(result.is_ok());
    
    let order_result = result.unwrap();
    assert!(order_result.status == OrderStatus::Submitted || order_result.status == OrderStatus::Filled);
    
    // If order is still active, cancel it
    if order_result.status == OrderStatus::Submitted {
        let cancel_result = engine.cancel_order(&order_result.order_id).await;
        assert!(cancel_result.is_ok());
    }
    
    // Disconnect
    engine.disconnect().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_live_trading_emergency_stop() {
    let wallet = create_test_wallet();
    let risk_config = create_test_risk_config();
    let api_config = create_test_api_config();
    
    let mut engine = LiveTradingEngine::new(wallet, risk_config, api_config).await.unwrap();
    
    // Connect
    engine.connect().await.unwrap();
    
    // Subscribe to market data
    if let Some(data_stream) = &engine.real_time_data {
        let mut stream = data_stream.lock().unwrap();
        let result = stream.subscribe_to_ticker("BTC").await;
        assert!(result.is_ok());
    }
    
    // Wait for market data
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Create a limit buy order 5% below current price
    let market_data = engine.get_market_data("BTC").unwrap();
    let limit_price = market_data.price * 0.95;
    let order = create_test_order("BTC", OrderSide::Buy, 0.0001, Some(limit_price));
    
    // Submit order
    let result = engine.execute_order(order).await;
    assert!(result.is_ok());
    
    // Activate emergency stop
    let result = engine.emergency_stop().await;
    assert!(result.is_ok());
    
    // Check emergency stop is active
    assert!(engine.is_emergency_stop_active());
    
    // Try to submit another order (should fail)
    let order = create_test_order("BTC", OrderSide::Buy, 0.0001, None);
    let result = engine.execute_order(order).await;
    assert!(result.is_err());
    
    match result {
        Err(LiveTradingError::EmergencyStop) => {},
        _ => panic!("Expected EmergencyStop error"),
    }
    
    // Disconnect
    engine.disconnect().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_live_trading_risk_management() {
    let wallet = create_test_wallet();
    
    // Create strict risk config
    let risk_config = RiskConfig {
        max_position_size_pct: 0.001,     // 0.1% of portfolio
        max_daily_loss_pct: 0.01,         // 1% max daily loss
        stop_loss_pct: 0.02,              // 2% stop loss
        take_profit_pct: 0.05,            // 5% take profit
        max_leverage: 1.5,                // 1.5x max leverage
        max_concentration_pct: 0.05,      // 5% max concentration
        max_position_correlation: 0.3,    // 0.3 maximum correlation
        max_portfolio_volatility_pct: 0.05, // 5% maximum portfolio volatility
        volatility_sizing_factor: 0.2,    // 20% volatility-based position sizing
        max_drawdown_pct: 0.05,           // 5% maximum drawdown
    };
    
    let api_config = create_test_api_config();
    
    let mut engine = LiveTradingEngine::new(wallet, risk_config, api_config).await.unwrap();
    
    // Connect
    engine.connect().await.unwrap();
    
    // Subscribe to market data
    if let Some(data_stream) = &engine.real_time_data {
        let mut stream = data_stream.lock().unwrap();
        let result = stream.subscribe_to_ticker("BTC").await;
        assert!(result.is_ok());
    }
    
    // Wait for market data
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Try to create an order that exceeds position size limits
    let order = create_test_order("BTC", OrderSide::Buy, 1.0, None); // Large order
    
    // Execute order (should fail due to risk limits)
    let result = engine.execute_order(order).await;
    assert!(result.is_err());
    
    match result {
        Err(LiveTradingError::RiskError(_)) => {},
        _ => panic!("Expected RiskError"),
    }
    
    // Disconnect
    engine.disconnect().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_live_trading_strategy_execution() {
    let wallet = create_test_wallet();
    let risk_config = create_test_risk_config();
    let api_config = create_test_api_config();
    
    let mut engine = LiveTradingEngine::new(wallet, risk_config, api_config).await.unwrap();
    
    // Connect
    engine.connect().await.unwrap();
    
    // Create strategy
    let strategy = Box::new(MockStrategy::new("TestStrategy", true));
    
    // Start strategy execution (with very small order size)
    let result = engine.start_trading(strategy, 0.0001).await;
    assert!(result.is_ok());
    
    // Let it run for a short time
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    
    // Stop strategy execution
    engine.stop_trading().await.unwrap();
    
    // Check if any orders were executed
    let order_history = engine.get_order_history();
    println!("Orders executed: {}", order_history.len());
    
    // Disconnect
    engine.disconnect().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_live_trading_multi_asset() {
    let wallet = create_test_wallet();
    let risk_config = create_test_risk_config();
    let api_config = create_test_api_config();
    
    let mut engine = LiveTradingEngine::new(wallet, risk_config, api_config).await.unwrap();
    
    // Connect
    engine.connect().await.unwrap();
    
    // Subscribe to multiple assets
    if let Some(data_stream) = &engine.real_time_data {
        let mut stream = data_stream.lock().unwrap();
        stream.subscribe_to_ticker("BTC").await.unwrap();
        stream.subscribe_to_ticker("ETH").await.unwrap();
        stream.subscribe_to_ticker("SOL").await.unwrap();
    }
    
    // Wait for market data
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Execute small orders for multiple assets
    let btc_order = create_test_order("BTC", OrderSide::Buy, 0.0001, None);
    let eth_order = create_test_order("ETH", OrderSide::Buy, 0.001, None);
    let sol_order = create_test_order("SOL", OrderSide::Buy, 0.01, None);
    
    let btc_result = engine.execute_order(btc_order).await;
    let eth_result = engine.execute_order(eth_order).await;
    let sol_result = engine.execute_order(sol_order).await;
    
    // Check results
    println!("BTC order result: {:?}", btc_result.is_ok());
    println!("ETH order result: {:?}", eth_result.is_ok());
    println!("SOL order result: {:?}", sol_result.is_ok());
    
    // Close positions
    if btc_result.is_ok() {
        let close_order = create_test_order("BTC", OrderSide::Sell, 0.0001, None);
        engine.execute_order(close_order).await.unwrap();
    }
    
    if eth_result.is_ok() {
        let close_order = create_test_order("ETH", OrderSide::Sell, 0.001, None);
        engine.execute_order(close_order).await.unwrap();
    }
    
    if sol_result.is_ok() {
        let close_order = create_test_order("SOL", OrderSide::Sell, 0.01, None);
        engine.execute_order(close_order).await.unwrap();
    }
    
    // Disconnect
    engine.disconnect().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_live_trading_order_retry() {
    let wallet = create_test_wallet();
    let risk_config = create_test_risk_config();
    let api_config = create_test_api_config();
    
    let mut engine = LiveTradingEngine::new(wallet, risk_config, api_config).await.unwrap();
    
    // Set custom retry policy
    let retry_policy = RetryPolicy {
        max_attempts: 3,
        initial_delay_ms: 500,
        backoff_factor: 2.0,
        max_delay_ms: 5000,
    };
    engine.set_retry_policy(retry_policy);
    
    // Connect
    engine.connect().await.unwrap();
    
    // Subscribe to market data
    if let Some(data_stream) = &engine.real_time_data {
        let mut stream = data_stream.lock().unwrap();
        stream.subscribe_to_ticker("BTC").await.unwrap();
    }
    
    // Wait for market data
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Create an order with invalid parameters to trigger retry
    // Note: This is implementation-specific and may need adjustment
    let invalid_order = OrderRequest {
        symbol: "INVALID".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Market,
        quantity: 0.0001,
        price: None,
        reduce_only: false,
        time_in_force: TimeInForce::GoodTillCancel,
    };
    
    // Execute order (should fail and trigger retry)
    let result = engine.execute_order(invalid_order).await;
    
    // Check result
    println!("Order result: {:?}", result);
    
    // Disconnect
    engine.disconnect().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_live_trading_safety_circuit_breakers() {
    let wallet = create_test_wallet();
    let risk_config = create_test_risk_config();
    let api_config = create_test_api_config();
    
    let mut engine = LiveTradingEngine::new(wallet, risk_config, api_config).await.unwrap();
    
    // Set custom safety circuit breaker config
    let safety_config = SafetyCircuitBreakerConfig {
        max_consecutive_failed_orders: 2,
        max_order_failure_rate: 0.5,
        order_failure_rate_window: 5,
        max_position_drawdown_pct: 0.1,
        max_account_drawdown_pct: 0.05,
        max_price_deviation_pct: 0.03,
        price_deviation_window_sec: 30,
        max_critical_alerts: 2,
        critical_alerts_window: 5,
    };
    engine.set_safety_circuit_breaker_config(safety_config);
    
    // Connect
    engine.connect().await.unwrap();
    
    // Simulate consecutive failed orders to trigger circuit breaker
    engine.update_order_result(false);
    engine.update_order_result(false);
    
    // Check safety circuit breakers
    let result = engine.check_safety_circuit_breakers();
    assert!(result.is_err());
    
    // Check emergency stop is active
    assert!(engine.is_emergency_stop_active());
    
    // Disconnect
    engine.disconnect().await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_live_trading_alert_system() {
    let wallet = create_test_wallet();
    let risk_config = create_test_risk_config();
    let api_config = create_test_api_config();
    
    let mut engine = LiveTradingEngine::new(wallet, risk_config, api_config).await.unwrap();
    
    // Connect
    engine.connect().await.unwrap();
    
    // Send alerts of different levels
    engine.send_alert(AlertLevel::Info, "Informational message", Some("BTC"), None);
    engine.send_alert(AlertLevel::Warning, "Warning message", Some("ETH"), None);
    engine.send_alert(AlertLevel::Error, "Error message", Some("BTC"), Some("order123"));
    
    // Send critical alerts
    engine.send_alert(AlertLevel::Critical, "Critical message 1", None, None);
    engine.send_alert(AlertLevel::Critical, "Critical message 2", None, None);
    
    // Check if emergency stop was triggered
    assert!(engine.is_emergency_stop_active());
    
    // Disconnect
    engine.disconnect().await.unwrap();
}

// Integration test for all safety mechanisms working together
#[tokio::test]
#[ignore]
async fn test_live_trading_safety_mechanisms_integration() {
    let wallet = create_test_wallet();
    let risk_config = create_test_risk_config();
    let api_config = create_test_api_config();
    
    let mut engine = LiveTradingEngine::new(wallet, risk_config, api_config).await.unwrap();
    
    // Connect
    engine.connect().await.unwrap();
    
    // Subscribe to market data
    if let Some(data_stream) = &engine.real_time_data {
        let mut stream = data_stream.lock().unwrap();
        stream.subscribe_to_ticker("BTC").await.unwrap();
    }
    
    // Wait for market data
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Execute a small order
    let order = create_test_order("BTC", OrderSide::Buy, 0.0001, None);
    let result = engine.execute_order(order).await;
    
    if result.is_ok() {
        println!("Order executed successfully");
        
        // Check if stop-loss was registered
        let stop_losses = engine.risk_manager.get_stop_losses();
        assert!(!stop_losses.is_empty());
        
        // Check if take-profit was registered
        let take_profits = engine.risk_manager.get_take_profits();
        assert!(!take_profits.is_empty());
        
        // Close position
        let close_order = create_test_order("BTC", OrderSide::Sell, 0.0001, None);
        engine.execute_order(close_order).await.unwrap();
    } else {
        println!("Order execution failed: {:?}", result.err());
    }
    
    // Disconnect
    engine.disconnect().await.unwrap();
}