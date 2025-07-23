use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use chrono::{DateTime, FixedOffset, Utc};
use tokio::test;

use crate::trading_mode::{ApiConfig, RiskConfig, SlippageConfig};
use crate::unified_data::{
    Position, OrderRequest, OrderResult, MarketData, 
    OrderSide, OrderType, TimeInForce, OrderStatus,
    TradingStrategy, Signal
};
use crate::risk_manager::{RiskManager, RiskError, RiskOrder};

// Helper function to create test market data
fn create_test_market_data(symbol: &str, price: f64) -> MarketData {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    MarketData::new(
        symbol,
        price,
        price * 0.999, // Bid slightly lower
        price * 1.001, // Ask slightly higher
        100.0,         // Volume
        now,
    )
}

#[test]
fn test_risk_manager_position_size_limits() {
    // Create risk config with strict position size limits
    let risk_config = RiskConfig {
        max_position_size_pct: 0.01, // 1% of portfolio
        max_daily_loss_pct: 0.05,
        stop_loss_pct: 0.05,
        take_profit_pct: 0.1,
        max_leverage: 2.0,
        max_concentration_pct: 0.1,
        max_position_correlation: 0.5,
        max_portfolio_volatility_pct: 0.1,
        volatility_sizing_factor: 0.3,
        max_drawdown_pct: 0.1,
    };
    
    // Create risk manager with $10,000 account
    let mut risk_manager = RiskManager::new(risk_config, 10000.0);
    
    // Create positions
    let mut positions = HashMap::new();
    
    // Test valid order (within limits)
    let small_order = OrderRequest::market("BTC", OrderSide::Buy, 0.01);
    let result = risk_manager.validate_order(&small_order, &positions);
    assert!(result.is_ok());
    
    // Test order that exceeds position size limits
    // BTC at $50,000 * 0.5 = $25,000 which is > 1% of $10,000
    let large_order = OrderRequest::market("BTC", OrderSide::Buy, 0.5);
    let result = risk_manager.validate_order(&large_order, &positions);
    assert!(result.is_err());
    
    match result {
        Err(RiskError::PositionSizeLimitExceeded { symbol, size, limit }) => {
            assert_eq!(symbol, "BTC");
            assert_eq!(size, 0.5);
            assert!(limit < 0.5);
        },
        _ => panic!("Expected PositionSizeLimitExceeded error"),
    }
}

#[test]
fn test_risk_manager_leverage_limits() {
    // Create risk config with strict leverage limits
    let risk_config = RiskConfig {
        max_position_size_pct: 0.5, // 50% of portfolio
        max_daily_loss_pct: 0.05,
        stop_loss_pct: 0.05,
        take_profit_pct: 0.1,
        max_leverage: 1.5, // 1.5x max leverage
        max_concentration_pct: 0.5,
        max_position_correlation: 0.5,
        max_portfolio_volatility_pct: 0.1,
        volatility_sizing_factor: 0.3,
        max_drawdown_pct: 0.1,
    };
    
    // Create risk manager with $10,000 account
    let mut risk_manager = RiskManager::new(risk_config, 10000.0);
    
    // Create positions with existing leverage
    let mut positions = HashMap::new();
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Add position that uses 1x leverage
    positions.insert("ETH".to_string(), Position::new(
        "ETH",
        1.0,
        3000.0,
        3000.0,
        now,
    ));
    
    // Test order that would exceed leverage limits
    // Current position: $3,000 in ETH
    // Account value: $10,000
    // Adding 0.2 BTC at $50,000 = $10,000 more exposure
    // Total exposure would be $13,000 / $10,000 = 1.3x leverage
    let btc_order = OrderRequest::market("BTC", OrderSide::Buy, 0.2);
    let result = risk_manager.validate_order(&btc_order, &positions);
    assert!(result.is_ok()); // Should be ok as 1.3x < 1.5x
    
    // Test order that would exceed leverage limits
    // Adding 0.3 BTC at $50,000 = $15,000 more exposure
    // Total exposure would be $18,000 / $10,000 = 1.8x leverage
    let large_btc_order = OrderRequest::market("BTC", OrderSide::Buy, 0.3);
    let result = risk_manager.validate_order(&large_btc_order, &positions);
    assert!(result.is_err());
    
    match result {
        Err(RiskError::LeverageLimitExceeded { current, limit, would_be }) => {
            assert_eq!(limit, 1.5);
            assert!(would_be > 1.5);
        },
        _ => panic!("Expected LeverageLimitExceeded error"),
    }
}

#[test]
fn test_risk_manager_concentration_limits() {
    // Create risk config with strict concentration limits
    let risk_config = RiskConfig {
        max_position_size_pct: 0.5, // 50% of portfolio
        max_daily_loss_pct: 0.05,
        stop_loss_pct: 0.05,
        take_profit_pct: 0.1,
        max_leverage: 3.0,
        max_concentration_pct: 0.2, // 20% max in one asset
        max_position_correlation: 0.5,
        max_portfolio_volatility_pct: 0.1,
        volatility_sizing_factor: 0.3,
        max_drawdown_pct: 0.1,
    };
    
    // Create risk manager with $10,000 account
    let mut risk_manager = RiskManager::new(risk_config, 10000.0);
    
    // Create positions
    let mut positions = HashMap::new();
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Test order that would exceed concentration limits
    // 0.05 BTC at $50,000 = $2,500 which is 25% of $10,000
    let btc_order = OrderRequest::market("BTC", OrderSide::Buy, 0.05);
    let result = risk_manager.validate_order(&btc_order, &positions);
    assert!(result.is_err());
    
    match result {
        Err(RiskError::ConcentrationLimitExceeded { symbol, concentration, limit }) => {
            assert_eq!(symbol, "BTC");
            assert!(concentration > 0.2);
            assert_eq!(limit, 0.2);
        },
        _ => panic!("Expected ConcentrationLimitExceeded error"),
    }
    
    // Test order within concentration limits
    let small_btc_order = OrderRequest::market("BTC", OrderSide::Buy, 0.01);
    let result = risk_manager.validate_order(&small_btc_order, &positions);
    assert!(result.is_ok());
}

#[test]
fn test_risk_manager_daily_loss_limits() {
    // Create risk config with strict daily loss limits
    let risk_config = RiskConfig {
        max_position_size_pct: 0.5,
        max_daily_loss_pct: 0.02, // 2% max daily loss
        stop_loss_pct: 0.05,
        take_profit_pct: 0.1,
        max_leverage: 3.0,
        max_concentration_pct: 0.5,
        max_position_correlation: 0.5,
        max_portfolio_volatility_pct: 0.1,
        volatility_sizing_factor: 0.3,
        max_drawdown_pct: 0.1,
    };
    
    // Create risk manager with $10,000 account
    let mut risk_manager = RiskManager::new(risk_config, 10000.0);
    
    // Simulate daily loss of 1.5% ($150)
    risk_manager.update_daily_pnl(-150.0);
    
    // Test order that would be allowed
    let small_order = OrderRequest::market("BTC", OrderSide::Buy, 0.01);
    let result = risk_manager.validate_order(&small_order, &HashMap::new());
    assert!(result.is_ok());
    
    // Simulate additional loss of 1% ($100)
    risk_manager.update_daily_pnl(-100.0);
    
    // Test order after exceeding daily loss limit
    let another_order = OrderRequest::market("BTC", OrderSide::Buy, 0.01);
    let result = risk_manager.validate_order(&another_order, &HashMap::new());
    assert!(result.is_err());
    
    match result {
        Err(RiskError::DailyLossLimitExceeded { loss, limit }) => {
            assert!(loss > 0.02);
            assert_eq!(limit, 0.02);
        },
        _ => panic!("Expected DailyLossLimitExceeded error"),
    }
}

#[test]
fn test_risk_manager_stop_loss_generation() {
    // Create risk config
    let risk_config = RiskConfig {
        max_position_size_pct: 0.5,
        max_daily_loss_pct: 0.05,
        stop_loss_pct: 0.05, // 5% stop loss
        take_profit_pct: 0.1, // 10% take profit
        max_leverage: 3.0,
        max_concentration_pct: 0.5,
        max_position_correlation: 0.5,
        max_portfolio_volatility_pct: 0.1,
        volatility_sizing_factor: 0.3,
        max_drawdown_pct: 0.1,
    };
    
    // Create risk manager
    let mut risk_manager = RiskManager::new(risk_config, 10000.0);
    
    // Create a position
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let position = Position::new(
        "BTC",
        0.1,
        50000.0,
        50000.0,
        now,
    );
    
    // Generate stop loss
    let stop_loss = risk_manager.generate_stop_loss(&position, "order123");
    assert!(stop_loss.is_some());
    
    let stop_loss = stop_loss.unwrap();
    assert_eq!(stop_loss.symbol, "BTC");
    assert_eq!(stop_loss.position_size, 0.1);
    assert_eq!(stop_loss.side, OrderSide::Sell); // Sell to close long position
    assert_eq!(stop_loss.trigger_price, 50000.0 * 0.95); // 5% below entry
    
    // Generate take profit
    let take_profit = risk_manager.generate_take_profit(&position, "order123");
    assert!(take_profit.is_some());
    
    let take_profit = take_profit.unwrap();
    assert_eq!(take_profit.symbol, "BTC");
    assert_eq!(take_profit.position_size, 0.1);
    assert_eq!(take_profit.side, OrderSide::Sell); // Sell to close long position
    assert_eq!(take_profit.trigger_price, 50000.0 * 1.1); // 10% above entry
    
    // Test with short position
    let short_position = Position::new(
        "ETH",
        -1.0,
        3000.0,
        3000.0,
        now,
    );
    
    // Generate stop loss for short
    let stop_loss = risk_manager.generate_stop_loss(&short_position, "order456");
    assert!(stop_loss.is_some());
    
    let stop_loss = stop_loss.unwrap();
    assert_eq!(stop_loss.symbol, "ETH");
    assert_eq!(stop_loss.position_size, 1.0);
    assert_eq!(stop_loss.side, OrderSide::Buy); // Buy to close short position
    assert_eq!(stop_loss.trigger_price, 3000.0 * 1.05); // 5% above entry
}

#[test]
fn test_risk_manager_drawdown_protection() {
    // Create risk config with drawdown protection
    let risk_config = RiskConfig {
        max_position_size_pct: 0.5,
        max_daily_loss_pct: 0.05,
        stop_loss_pct: 0.05,
        take_profit_pct: 0.1,
        max_leverage: 3.0,
        max_concentration_pct: 0.5,
        max_position_correlation: 0.5,
        max_portfolio_volatility_pct: 0.1,
        volatility_sizing_factor: 0.3,
        max_drawdown_pct: 0.1, // 10% max drawdown
    };
    
    // Create risk manager with $10,000 account
    let mut risk_manager = RiskManager::new(risk_config, 10000.0);
    
    // Set peak account value
    risk_manager.update_account_value(11000.0); // Up 10%
    
    // Check if trading should continue
    assert!(!risk_manager.should_stop_trading());
    
    // Simulate drawdown
    risk_manager.update_account_value(10000.0); // Down to initial value
    assert!(!risk_manager.should_stop_trading());
    
    // Simulate drawdown exceeding limit
    risk_manager.update_account_value(9800.0); // Down 11% from peak
    assert!(risk_manager.should_stop_trading());
}