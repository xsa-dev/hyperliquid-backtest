use std::collections::HashMap;
use chrono::{DateTime, FixedOffset, Utc};

use crate::risk_manager::{RiskManager, RiskError, RiskOrder};
use crate::trading_mode::RiskConfig;
use crate::trading_mode_impl::{Position, OrderRequest, OrderSide, OrderType, TimeInForce};

// Helper function to create a test position
fn create_test_position(symbol: &str, size: f64, entry_price: f64, current_price: f64) -> Position {
    Position {
        symbol: symbol.to_string(),
        size,
        entry_price,
        current_price,
        unrealized_pnl: (current_price - entry_price) * size,
        realized_pnl: 0.0,
        funding_pnl: 0.0,
        timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
    }
}

// Helper function to create a test order
fn create_test_order(symbol: &str, side: OrderSide, quantity: f64, price: Option<f64>) -> OrderRequest {
    OrderRequest {
        symbol: symbol.to_string(),
        side,
        order_type: OrderType::Limit,
        quantity,
        price,
        reduce_only: false,
        time_in_force: TimeInForce::GoodTillCancel,
    }
}

#[test]
fn test_risk_manager_creation() {
    let config = RiskConfig::default();
    let portfolio_value = 10000.0;
    let risk_manager = RiskManager::new(config, portfolio_value);
    
    assert_eq!(risk_manager.config().max_position_size_pct, 0.1);
    assert_eq!(risk_manager.config().max_daily_loss_pct, 0.02);
    assert_eq!(risk_manager.config().stop_loss_pct, 0.05);
    assert_eq!(risk_manager.config().take_profit_pct, 0.1);
    assert_eq!(risk_manager.config().max_leverage, 3.0);
}

#[test]
fn test_position_size_validation() {
    let config = RiskConfig {
        max_position_size_pct: 0.1,  // 10% of portfolio
        max_daily_loss_pct: 0.02,    // 2% max daily loss
        stop_loss_pct: 0.05,         // 5% stop loss
        take_profit_pct: 0.1,        // 10% take profit
        max_leverage: 3.0,           // 3x max leverage
        max_concentration_pct: 0.25, // 25% max concentration in one asset class
        max_position_correlation: 0.7, // 0.7 maximum correlation between positions
        max_portfolio_volatility_pct: 0.2, // 20% maximum portfolio volatility
        volatility_sizing_factor: 0.5, // 50% volatility-based position sizing
        max_drawdown_pct: 0.15,     // 15% maximum drawdown before emergency stop
    };
    
    let portfolio_value = 10000.0;
    let mut risk_manager = RiskManager::new(config, portfolio_value);
    
    let mut positions = HashMap::new();
    
    // Test valid order within position size limit
    let order = create_test_order("BTC", OrderSide::Buy, 0.1, Some(9000.0));
    // Order value: 0.1 * 9000 = 900, which is < 10% of 10000 (1000)
    assert!(risk_manager.validate_order(&order, &positions).is_ok());
    
    // Test order exceeding position size limit
    let order = create_test_order("BTC", OrderSide::Buy, 0.2, Some(9000.0));
    // Order value: 0.2 * 9000 = 1800, which is > 10% of 10000 (1000)
    assert!(risk_manager.validate_order(&order, &positions).is_err());
    
    // Test with existing position
    positions.insert(
        "BTC".to_string(),
        create_test_position("BTC", 0.05, 8000.0, 9000.0)
    );
    
    // Test valid order with existing position
    let order = create_test_order("BTC", OrderSide::Buy, 0.05, Some(9000.0));
    // Existing position value: 0.05 * 9000 = 450
    // Order value: 0.05 * 9000 = 450
    // Total: 900, which is < 10% of 10000 (1000)
    assert!(risk_manager.validate_order(&order, &positions).is_ok());
    
    // Test order exceeding position size limit with existing position
    let order = create_test_order("BTC", OrderSide::Buy, 0.07, Some(9000.0));
    // Existing position value: 0.05 * 9000 = 450
    // Order value: 0.07 * 9000 = 630
    // Total: 1080, which is > 10% of 10000 (1000)
    assert!(risk_manager.validate_order(&order, &positions).is_err());
}

#[test]
fn test_leverage_validation() {
    let config = RiskConfig {
        max_position_size_pct: 0.5,  // 50% of portfolio (high to test leverage)
        max_daily_loss_pct: 0.02,    // 2% max daily loss
        stop_loss_pct: 0.05,         // 5% stop loss
        take_profit_pct: 0.1,        // 10% take profit
        max_leverage: 2.0,           // 2x max leverage
        max_concentration_pct: 0.25, // 25% max concentration in one asset class
        max_position_correlation: 0.7, // 0.7 maximum correlation between positions
        max_portfolio_volatility_pct: 0.2, // 20% maximum portfolio volatility
        volatility_sizing_factor: 0.5, // 50% volatility-based position sizing
        max_drawdown_pct: 0.15,     // 15% maximum drawdown before emergency stop
    };
    
    let portfolio_value = 10000.0;
    let mut risk_manager = RiskManager::new(config, portfolio_value);
    
    let mut positions = HashMap::new();
    positions.insert(
        "ETH".to_string(),
        create_test_position("ETH", 2.0, 1500.0, 1600.0)
    );
    // ETH position value: 2.0 * 1600 = 3200
    
    // Test valid order within leverage limit
    let order = create_test_order("BTC", OrderSide::Buy, 0.1, Some(9000.0));
    // Order value: 0.1 * 9000 = 900
    // Total position value: 3200 + 900 = 4100
    // Leverage: 4100 / 10000 = 0.41, which is < 2.0
    assert!(risk_manager.validate_order(&order, &positions).is_ok());
    
    // Test order exceeding leverage limit
    let order = create_test_order("BTC", OrderSide::Buy, 2.0, Some(9000.0));
    // Order value: 2.0 * 9000 = 18000
    // Total position value: 3200 + 18000 = 21200
    // Leverage: 21200 / 10000 = 2.12, which is > 2.0
    assert!(risk_manager.validate_order(&order, &positions).is_err());
}

#[test]
fn test_daily_loss_limit() {
    let config = RiskConfig {
        max_position_size_pct: 0.1,  // 10% of portfolio
        max_daily_loss_pct: 2.0,     // 2% max daily loss
        stop_loss_pct: 0.05,         // 5% stop loss
        take_profit_pct: 0.1,        // 10% take profit
        max_leverage: 3.0,           // 3x max leverage
        max_concentration_pct: 0.25, // 25% max concentration in one asset class
        max_position_correlation: 0.7, // 0.7 maximum correlation between positions
        max_portfolio_volatility_pct: 0.2, // 20% maximum portfolio volatility
        volatility_sizing_factor: 0.5, // 50% volatility-based position sizing
        max_drawdown_pct: 0.15,     // 15% maximum drawdown before emergency stop
    };
    
    let portfolio_value = 10000.0;
    let mut risk_manager = RiskManager::new(config, portfolio_value);
    
    // Update portfolio value with small loss (1%)
    assert!(risk_manager.update_portfolio_value(9900.0, -100.0).is_ok());
    
    // Verify daily loss is tracked correctly
    let (daily_loss_pct, _, _) = risk_manager.daily_risk_metrics();
    assert_eq!(daily_loss_pct, 1.0);
    
    // Update portfolio value with loss exceeding daily limit (3% total)
    assert!(risk_manager.update_portfolio_value(9700.0, -200.0).is_err());
    
    // Verify trading should be stopped
    assert!(risk_manager.should_stop_trading());
}

#[test]
fn test_stop_loss_generation() {
    let config = RiskConfig {
        max_position_size_pct: 0.1,  // 10% of portfolio
        max_daily_loss_pct: 2.0,     // 2% max daily loss
        stop_loss_pct: 0.05,         // 5% stop loss
        take_profit_pct: 0.1,        // 10% take profit
        max_leverage: 3.0,           // 3x max leverage
        max_concentration_pct: 0.25, // 25% max concentration in one asset class
        max_position_correlation: 0.7, // 0.7 maximum correlation between positions
        max_portfolio_volatility_pct: 0.2, // 20% maximum portfolio volatility
        volatility_sizing_factor: 0.5, // 50% volatility-based position sizing
        max_drawdown_pct: 0.15,     // 15% maximum drawdown before emergency stop
    };
    
    let portfolio_value = 10000.0;
    let risk_manager = RiskManager::new(config, portfolio_value);
    
    // Test stop loss for long position
    let long_position = create_test_position("BTC", 0.1, 10000.0, 10000.0);
    let stop_loss = risk_manager.generate_stop_loss(&long_position, "order1").unwrap();
    
    assert_eq!(stop_loss.symbol, "BTC");
    assert!(matches!(stop_loss.side, OrderSide::Sell));
    assert!(matches!(stop_loss.order_type, OrderType::StopMarket));
    assert_eq!(stop_loss.quantity, 0.1);
    assert_eq!(stop_loss.trigger_price, 9500.0); // 5% below entry price
    
    // Test stop loss for short position
    let short_position = create_test_position("BTC", -0.1, 10000.0, 10000.0);
    let stop_loss = risk_manager.generate_stop_loss(&short_position, "order2").unwrap();
    
    assert_eq!(stop_loss.symbol, "BTC");
    assert!(matches!(stop_loss.side, OrderSide::Buy));
    assert!(matches!(stop_loss.order_type, OrderType::StopMarket));
    assert_eq!(stop_loss.quantity, 0.1);
    assert_eq!(stop_loss.trigger_price, 10500.0); // 5% above entry price
}

#[test]
fn test_take_profit_generation() {
    let config = RiskConfig {
        max_position_size_pct: 0.1,  // 10% of portfolio
        max_daily_loss_pct: 2.0,     // 2% max daily loss
        stop_loss_pct: 0.05,         // 5% stop loss
        take_profit_pct: 0.1,        // 10% take profit
        max_leverage: 3.0,           // 3x max leverage
        max_concentration_pct: 0.25, // 25% max concentration in one asset class
        max_position_correlation: 0.7, // 0.7 maximum correlation between positions
        max_portfolio_volatility_pct: 0.2, // 20% maximum portfolio volatility
        volatility_sizing_factor: 0.5, // 50% volatility-based position sizing
        max_drawdown_pct: 0.15,     // 15% maximum drawdown before emergency stop
    };
    
    let portfolio_value = 10000.0;
    let risk_manager = RiskManager::new(config, portfolio_value);
    
    // Test take profit for long position
    let long_position = create_test_position("BTC", 0.1, 10000.0, 10000.0);
    let take_profit = risk_manager.generate_take_profit(&long_position, "order1").unwrap();
    
    assert_eq!(take_profit.symbol, "BTC");
    assert!(matches!(take_profit.side, OrderSide::Sell));
    assert!(matches!(take_profit.order_type, OrderType::TakeProfitMarket));
    assert_eq!(take_profit.quantity, 0.1);
    assert_eq!(take_profit.trigger_price, 11000.0); // 10% above entry price
    
    // Test take profit for short position
    let short_position = create_test_position("BTC", -0.1, 10000.0, 10000.0);
    let take_profit = risk_manager.generate_take_profit(&short_position, "order2").unwrap();
    
    assert_eq!(take_profit.symbol, "BTC");
    assert!(matches!(take_profit.side, OrderSide::Buy));
    assert!(matches!(take_profit.order_type, OrderType::TakeProfitMarket));
    assert_eq!(take_profit.quantity, 0.1);
    assert_eq!(take_profit.trigger_price, 9000.0); // 10% below entry price
}

#[test]
fn test_risk_orders_triggering() {
    let config = RiskConfig {
        max_position_size_pct: 0.1,
        max_daily_loss_pct: 2.0,
        stop_loss_pct: 0.05,
        take_profit_pct: 0.1,
        max_leverage: 3.0,
        max_concentration_pct: 0.25,
        max_position_correlation: 0.7,
        max_portfolio_volatility_pct: 0.2,
        volatility_sizing_factor: 0.5,
        max_drawdown_pct: 0.15,
        max_concentration_pct: 0.25,
        max_position_correlation: 0.7,
        max_portfolio_volatility_pct: 0.2,
        volatility_sizing_factor: 0.5,
        max_drawdown_pct: 0.15,
    };
    
    let portfolio_value = 10000.0;
    let mut risk_manager = RiskManager::new(config, portfolio_value);
    
    // Create and register a stop loss order
    let long_position = create_test_position("BTC", 0.1, 10000.0, 10000.0);
    let stop_loss = risk_manager.generate_stop_loss(&long_position, "order1").unwrap();
    risk_manager.register_stop_loss(stop_loss);
    
    // Create and register a take profit order
    let take_profit = risk_manager.generate_take_profit(&long_position, "order1").unwrap();
    risk_manager.register_take_profit(take_profit);
    
    // Test no orders triggered at current price
    let mut current_prices = HashMap::new();
    current_prices.insert("BTC".to_string(), 10000.0);
    let triggered = risk_manager.check_risk_orders(&current_prices);
    assert_eq!(triggered.len(), 0);
    
    // Test stop loss triggered
    current_prices.insert("BTC".to_string(), 9400.0); // Below stop loss price
    let triggered = risk_manager.check_risk_orders(&current_prices);
    assert_eq!(triggered.len(), 1);
    assert!(triggered[0].is_stop_loss);
    
    // Register new orders
    let long_position = create_test_position("BTC", 0.1, 10000.0, 10000.0);
    let stop_loss = risk_manager.generate_stop_loss(&long_position, "order2").unwrap();
    risk_manager.register_stop_loss(stop_loss);
    let take_profit = risk_manager.generate_take_profit(&long_position, "order2").unwrap();
    risk_manager.register_take_profit(take_profit);
    
    // Test take profit triggered
    current_prices.insert("BTC".to_string(), 11100.0); // Above take profit price
    let triggered = risk_manager.check_risk_orders(&current_prices);
    assert_eq!(triggered.len(), 1);
    assert!(triggered[0].is_take_profit);
}

#[test]
fn test_emergency_stop() {
    let config = RiskConfig::default();
    let portfolio_value = 10000.0;
    let mut risk_manager = RiskManager::new(config, portfolio_value);
    
    // Initially, emergency stop should be false
    assert!(!risk_manager.should_stop_trading());
    
    // Activate emergency stop
    risk_manager.activate_emergency_stop();
    assert!(risk_manager.should_stop_trading());
    
    // Orders should be rejected when emergency stop is active
    let positions = HashMap::new();
    let order = create_test_order("BTC", OrderSide::Buy, 0.1, Some(10000.0));
    assert!(risk_manager.validate_order(&order, &positions).is_err());
    
    // Deactivate emergency stop
    risk_manager.deactivate_emergency_stop();
    assert!(!risk_manager.should_stop_trading());
    
    // Orders should be accepted again
    assert!(risk_manager.validate_order(&order, &positions).is_ok());
}

#[test]
fn test_margin_requirements() {
    let config = RiskConfig {
        max_position_size_pct: 0.5,
        max_daily_loss_pct: 2.0,
        stop_loss_pct: 0.05,
        take_profit_pct: 0.1,
        max_leverage: 5.0,  // 5x max leverage
        max_concentration_pct: 0.25,
        max_position_correlation: 0.7,
        max_portfolio_volatility_pct: 0.2,
        volatility_sizing_factor: 0.5,
        max_drawdown_pct: 0.15,
    };
    
    let portfolio_value = 10000.0;
    let mut risk_manager = RiskManager::new(config, portfolio_value);
    
    // Test margin calculation
    let position_value = 20000.0;
    let required_margin = risk_manager.calculate_required_margin(position_value);
    assert_eq!(required_margin, 4000.0); // 20000 / 5 = 4000
    
    // Test with insufficient margin
    risk_manager.update_available_margin(3000.0);
    
    let positions = HashMap::new();
    let order = create_test_order("BTC", OrderSide::Buy, 0.5, Some(10000.0));
    // Order value: 0.5 * 10000 = 5000
    // Required margin: 5000 / 5 = 1000
    // Available margin: 3000
    assert!(risk_manager.validate_order(&order, &positions).is_ok());
    
    // Test with insufficient margin
    let order = create_test_order("BTC", OrderSide::Buy, 2.0, Some(10000.0));
    // Order value: 2.0 * 10000 = 20000
    // Required margin: 20000 / 5 = 4000
    // Available margin: 3000
    assert!(risk_manager.validate_order(&order, &positions).is_err());
}

#[test]
fn test_risk_config_update() {
    let config = RiskConfig::default();
    let portfolio_value = 10000.0;
    let mut risk_manager = RiskManager::new(config, portfolio_value);
    
    // Initial config
    assert_eq!(risk_manager.config().max_position_size_pct, 0.1);
    
    // Update config
    let new_config = RiskConfig {
        max_position_size_pct: 0.2,
        max_daily_loss_pct: 0.03,
        stop_loss_pct: 0.07,
        take_profit_pct: 0.15,
        max_leverage: 4.0,
        max_concentration_pct: 0.3,
        max_position_correlation: 0.8,
        max_portfolio_volatility_pct: 0.25,
        volatility_sizing_factor: 0.6,
        max_drawdown_pct: 0.2,
    };
    
    risk_manager.update_config(new_config);
    
    // Verify updated config
    assert_eq!(risk_manager.config().max_position_size_pct, 0.2);
    assert_eq!(risk_manager.config().max_daily_loss_pct, 0.03);
    assert_eq!(risk_manager.config().stop_loss_pct, 0.07);
    assert_eq!(risk_manager.config().take_profit_pct, 0.15);
    assert_eq!(risk_manager.config().max_leverage, 4.0);
}