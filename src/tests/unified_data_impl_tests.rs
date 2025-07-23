use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use std::collections::HashMap;

use crate::unified_data_impl::{
    Position, OrderRequest, OrderResult, MarketData, Signal, SignalDirection,
    OrderSide, OrderType, TimeInForce, OrderStatus, TradingConfig, RiskConfig,
    SlippageConfig, ApiConfig, OrderBookLevel, OrderBookSnapshot, Trade
};

#[test]
fn test_position_creation_and_methods() {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Create a long position
    let mut position = Position::new("BTC", 1.0, 50000.0, 51000.0, now);
    
    // Test basic properties
    assert_eq!(position.symbol, "BTC");
    assert_eq!(position.size, 1.0);
    assert_eq!(position.entry_price, 50000.0);
    assert_eq!(position.current_price, 51000.0);
    assert_eq!(position.unrealized_pnl, 1000.0); // (51000 - 50000) * 1.0
    assert_eq!(position.realized_pnl, 0.0);
    assert_eq!(position.funding_pnl, 0.0);
    assert_eq!(position.timestamp, now);
    
    // Test position methods
    assert!(position.is_long());
    assert!(!position.is_short());
    assert!(!position.is_flat());
    assert_eq!(position.notional_value(), 51000.0); // 1.0 * 51000.0
    assert_eq!(position.total_pnl(), 1000.0); // unrealized + realized + funding
    
    // Update price and check PnL changes
    position.update_price(52000.0);
    assert_eq!(position.current_price, 52000.0);
    assert_eq!(position.unrealized_pnl, 2000.0); // (52000 - 50000) * 1.0
    
    // Apply funding payment
    position.apply_funding_payment(100.0);
    assert_eq!(position.funding_pnl, 100.0);
    assert_eq!(position.total_pnl(), 2100.0); // 2000 + 0 + 100
}

#[test]
fn test_order_request_creation_and_validation() {
    // Create a market order
    let market_order = OrderRequest::market("BTC", OrderSide::Buy, 1.0);
    
    // Test basic properties
    assert_eq!(market_order.symbol, "BTC");
    assert_eq!(market_order.side, OrderSide::Buy);
    assert_eq!(market_order.order_type, OrderType::Market);
    assert_eq!(market_order.quantity, 1.0);
    assert_eq!(market_order.price, None);
    assert_eq!(market_order.reduce_only, false);
    assert_eq!(market_order.time_in_force, TimeInForce::GoodTillCancel);
    
    // Create a limit order
    let limit_order = OrderRequest::limit("ETH", OrderSide::Sell, 2.0, 3000.0)
        .reduce_only()
        .with_time_in_force(TimeInForce::FillOrKill)
        .with_client_order_id("test-order-123")
        .with_parameter("post_only", "true");
    
    // Test limit order properties
    assert_eq!(limit_order.symbol, "ETH");
    assert_eq!(limit_order.side, OrderSide::Sell);
    assert_eq!(limit_order.order_type, OrderType::Limit);
    assert_eq!(limit_order.quantity, 2.0);
    assert_eq!(limit_order.price, Some(3000.0));
    assert_eq!(limit_order.reduce_only, true);
    assert_eq!(limit_order.time_in_force, TimeInForce::FillOrKill);
    assert_eq!(limit_order.client_order_id, Some("test-order-123".to_string()));
    assert_eq!(limit_order.parameters.get("post_only"), Some(&"true".to_string()));
    
    // Test order validation
    assert!(market_order.validate().is_ok());
    assert!(limit_order.validate().is_ok());
    
    // Test validation failures
    let invalid_quantity = OrderRequest::market("BTC", OrderSide::Buy, 0.0);
    assert!(invalid_quantity.validate().is_err());
    
    let invalid_limit = OrderRequest {
        symbol: "BTC".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        quantity: 1.0,
        price: None, // Missing price for limit order
        reduce_only: false,
        time_in_force: TimeInForce::GoodTillCancel,
        stop_price: None,
        client_order_id: None,
        parameters: HashMap::new(),
    };
    assert!(invalid_limit.validate().is_err());
}

#[test]
fn test_market_data_creation_and_methods() {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let next_funding = now + chrono::Duration::hours(8);
    
    // Create basic market data
    let market_data = MarketData::new(
        "BTC",
        50000.0,
        49990.0,
        50010.0,
        100.0,
        now,
    );
    
    // Test basic properties
    assert_eq!(market_data.symbol, "BTC");
    assert_eq!(market_data.price, 50000.0);
    assert_eq!(market_data.bid, 49990.0);
    assert_eq!(market_data.ask, 50010.0);
    assert_eq!(market_data.volume, 100.0);
    assert_eq!(market_data.timestamp, now);
    
    // Test calculated properties
    assert_eq!(market_data.mid_price(), 50000.0); // (49990 + 50010) / 2
    assert_eq!(market_data.spread(), 20.0); // 50010 - 49990
    assert_eq!(market_data.spread_percentage(), 0.04); // (20 / 50000) * 100
    
    // Test builder methods
    let enhanced_data = market_data
        .with_funding_rate(0.0001, next_funding)
        .with_open_interest(1000.0)
        .with_24h_stats(5.0, 51000.0, 48000.0)
        .with_metadata("exchange", "hyperliquid");
    
    assert_eq!(enhanced_data.funding_rate, Some(0.0001));
    assert_eq!(enhanced_data.next_funding_time, Some(next_funding));
    assert_eq!(enhanced_data.open_interest, Some(1000.0));
    assert_eq!(enhanced_data.price_change_24h_pct, Some(5.0));
    assert_eq!(enhanced_data.high_24h, Some(51000.0));
    assert_eq!(enhanced_data.low_24h, Some(48000.0));
    assert_eq!(enhanced_data.metadata.get("exchange"), Some(&"hyperliquid".to_string()));
}

#[test]
fn test_trading_config_and_risk_config() {
    // Create risk config
    let risk_config = RiskConfig {
        max_position_size_pct: 0.1,
        max_daily_loss_pct: 0.02,
        stop_loss_pct: 0.05,
        take_profit_pct: 0.1,
        max_leverage: 3.0,
        max_positions: 5,
        max_drawdown_pct: 0.2,
        use_trailing_stop: true,
        trailing_stop_distance_pct: Some(0.02),
    };
    
    assert_eq!(risk_config.max_position_size_pct, 0.1);
    assert_eq!(risk_config.max_daily_loss_pct, 0.02);
    assert_eq!(risk_config.stop_loss_pct, 0.05);
    assert_eq!(risk_config.take_profit_pct, 0.1);
    assert_eq!(risk_config.max_leverage, 3.0);
    assert_eq!(risk_config.max_positions, 5);
    assert_eq!(risk_config.max_drawdown_pct, 0.2);
    assert_eq!(risk_config.use_trailing_stop, true);
    assert_eq!(risk_config.trailing_stop_distance_pct, Some(0.02));
    
    // Create trading config
    let mut trading_config = TradingConfig {
        initial_balance: 10000.0,
        risk_config: Some(risk_config),
        slippage_config: None,
        api_config: None,
        parameters: HashMap::new(),
    };
    
    assert_eq!(trading_config.initial_balance, 10000.0);
    assert!(trading_config.risk_config.is_some());
    assert!(trading_config.slippage_config.is_none());
    assert!(trading_config.api_config.is_none());
    
    // Add parameters
    trading_config.parameters.insert("backtest_mode".to_string(), "historical".to_string());
    assert_eq!(trading_config.parameters.get("backtest_mode"), Some(&"historical".to_string()));
}