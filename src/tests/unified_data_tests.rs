use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use std::collections::HashMap;

use crate::unified_data::{
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
    
    // Create a short position
    let mut short_position = Position::new("ETH", -2.0, 3000.0, 2900.0, now);
    
    // Test short position properties
    assert_eq!(short_position.symbol, "ETH");
    assert_eq!(short_position.size, -2.0);
    assert!(!short_position.is_long());
    assert!(short_position.is_short());
    assert!(!short_position.is_flat());
    assert_eq!(short_position.unrealized_pnl, 200.0); // (2900 - 3000) * -2.0
    assert_eq!(short_position.notional_value(), 5800.0); // 2.0 * 2900.0
    
    // Update price for short position
    short_position.update_price(2800.0);
    assert_eq!(short_position.unrealized_pnl, 400.0); // (2800 - 3000) * -2.0
    
    // Create a flat position
    let flat_position = Position::new("XRP", 0.0, 1.0, 1.0, now);
    assert!(flat_position.is_flat());
    assert_eq!(flat_position.unrealized_pnl, 0.0);
    assert_eq!(flat_position.notional_value(), 0.0);
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
    
    let invalid_stop = OrderRequest {
        symbol: "BTC".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::StopMarket,
        quantity: 1.0,
        price: None,
        reduce_only: false,
        time_in_force: TimeInForce::GoodTillCancel,
        stop_price: None, // Missing stop price
        client_order_id: None,
        parameters: HashMap::new(),
    };
    assert!(invalid_stop.validate().is_err());
}

#[test]
fn test_order_result_creation_and_methods() {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Create a new order result
    let mut order = OrderResult::new(
        "order-123",
        "BTC",
        OrderSide::Buy,
        OrderType::Market,
        1.0,
        now,
    );
    
    // Test initial state
    assert_eq!(order.order_id, "order-123");
    assert_eq!(order.symbol, "BTC");
    assert_eq!(order.side, OrderSide::Buy);
    assert_eq!(order.order_type, OrderType::Market);
    assert_eq!(order.requested_quantity, 1.0);
    assert_eq!(order.filled_quantity, 0.0);
    assert_eq!(order.average_price, None);
    assert_eq!(order.status, OrderStatus::Created);
    assert_eq!(order.timestamp, now);
    
    // Test status methods
    assert!(order.is_active());
    assert!(!order.is_complete());
    assert!(!order.is_filled());
    assert_eq!(order.fill_percentage(), 0.0);
    assert_eq!(order.filled_notional(), None);
    
    // Update order to partially filled
    order.status = OrderStatus::PartiallyFilled;
    order.filled_quantity = 0.5;
    order.average_price = Some(50000.0);
    
    assert!(order.is_active());
    assert!(!order.is_complete());
    assert!(order.is_filled());
    assert_eq!(order.fill_percentage(), 50.0);
    assert_eq!(order.filled_notional(), Some(25000.0)); // 0.5 * 50000.0
    
    // Update order to filled
    order.status = OrderStatus::Filled;
    order.filled_quantity = 1.0;
    
    assert!(!order.is_active());
    assert!(order.is_complete());
    assert!(order.is_filled());
    assert_eq!(order.fill_percentage(), 100.0);
    assert_eq!(order.filled_notional(), Some(50000.0)); // 1.0 * 50000.0
    
    // Test cancelled order
    let mut cancelled_order = OrderResult::new(
        "order-456",
        "ETH",
        OrderSide::Sell,
        OrderType::Limit,
        2.0,
        now,
    );
    cancelled_order.status = OrderStatus::Cancelled;
    
    assert!(!cancelled_order.is_active());
    assert!(cancelled_order.is_complete());
    assert!(!cancelled_order.is_filled());
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
fn test_order_book_snapshot() {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Create order book levels
    let bids = vec![
        OrderBookLevel { price: 49990.0, quantity: 1.0 },
        OrderBookLevel { price: 49980.0, quantity: 2.0 },
        OrderBookLevel { price: 49970.0, quantity: 3.0 },
    ];
    
    let asks = vec![
        OrderBookLevel { price: 50010.0, quantity: 1.0 },
        OrderBookLevel { price: 50020.0, quantity: 2.0 },
        OrderBookLevel { price: 50030.0, quantity: 3.0 },
    ];
    
    let order_book = OrderBookSnapshot {
        bids,
        asks,
        timestamp: now,
    };
    
    // Test order book properties
    assert_eq!(order_book.bids.len(), 3);
    assert_eq!(order_book.asks.len(), 3);
    assert_eq!(order_book.timestamp, now);
    
    // Test bid levels
    assert_eq!(order_book.bids[0].price, 49990.0);
    assert_eq!(order_book.bids[0].quantity, 1.0);
    assert_eq!(order_book.bids[1].price, 49980.0);
    assert_eq!(order_book.bids[1].quantity, 2.0);
    
    // Test ask levels
    assert_eq!(order_book.asks[0].price, 50010.0);
    assert_eq!(order_book.asks[0].quantity, 1.0);
    assert_eq!(order_book.asks[1].price, 50020.0);
    assert_eq!(order_book.asks[1].quantity, 2.0);
}

#[test]
fn test_trade_structure() {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    let trade = Trade {
        id: "trade-123".to_string(),
        price: 50000.0,
        quantity: 1.0,
        timestamp: now,
        side: Some(OrderSide::Buy),
    };
    
    assert_eq!(trade.id, "trade-123");
    assert_eq!(trade.price, 50000.0);
    assert_eq!(trade.quantity, 1.0);
    assert_eq!(trade.timestamp, now);
    assert_eq!(trade.side, Some(OrderSide::Buy));
}

#[test]
fn test_signal_structure() {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    let mut metadata = HashMap::new();
    metadata.insert("indicator".to_string(), "sma_cross".to_string());
    
    let signal = Signal {
        symbol: "BTC".to_string(),
        direction: SignalDirection::Buy,
        strength: 0.8,
        timestamp: now,
        metadata,
    };
    
    assert_eq!(signal.symbol, "BTC");
    assert_eq!(signal.direction, SignalDirection::Buy);
    assert_eq!(signal.strength, 0.8);
    assert_eq!(signal.timestamp, now);
    assert_eq!(signal.metadata.get("indicator"), Some(&"sma_cross".to_string()));
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
    
    // Create slippage config
    let slippage_config = SlippageConfig {
        base_slippage_pct: 0.0005,
        volume_impact_factor: 0.1,
        volatility_impact_factor: 0.2,
        random_slippage_max_pct: 0.001,
        simulated_latency_ms: 500,
        use_order_book: true,
        max_slippage_pct: 0.01,
    };
    
    assert_eq!(slippage_config.base_slippage_pct, 0.0005);
    assert_eq!(slippage_config.volume_impact_factor, 0.1);
    assert_eq!(slippage_config.volatility_impact_factor, 0.2);
    assert_eq!(slippage_config.random_slippage_max_pct, 0.001);
    assert_eq!(slippage_config.simulated_latency_ms, 500);
    assert_eq!(slippage_config.use_order_book, true);
    assert_eq!(slippage_config.max_slippage_pct, 0.01);
    
    // Create API config
    let api_config = ApiConfig {
        api_key: "test-key".to_string(),
        api_secret: "test-secret".to_string(),
        endpoint: "https://api.hyperliquid.io".to_string(),
        use_testnet: true,
        timeout_ms: 5000,
        rate_limit: Some(10.0),
        retry_attempts: 3,
        retry_delay_ms: 1000,
    };
    
    assert_eq!(api_config.api_key, "test-key");
    assert_eq!(api_config.api_secret, "test-secret");
    assert_eq!(api_config.endpoint, "https://api.hyperliquid.io");
    assert_eq!(api_config.use_testnet, true);
    assert_eq!(api_config.timeout_ms, 5000);
    assert_eq!(api_config.rate_limit, Some(10.0));
    assert_eq!(api_config.retry_attempts, 3);
    assert_eq!(api_config.retry_delay_ms, 1000);
    
    // Create trading config
    let mut trading_config = TradingConfig {
        initial_balance: 10000.0,
        risk_config: Some(risk_config),
        slippage_config: Some(slippage_config),
        api_config: Some(api_config),
        parameters: HashMap::new(),
    };
    
    assert_eq!(trading_config.initial_balance, 10000.0);
    assert!(trading_config.risk_config.is_some());
    assert!(trading_config.slippage_config.is_some());
    assert!(trading_config.api_config.is_some());
    
    // Add parameters
    trading_config.parameters.insert("backtest_mode".to_string(), "historical".to_string());
    assert_eq!(trading_config.parameters.get("backtest_mode"), Some(&"historical".to_string()));
}

#[test]
fn test_enum_display_implementations() {
    // Test OrderSide display
    assert_eq!(OrderSide::Buy.to_string(), "Buy");
    assert_eq!(OrderSide::Sell.to_string(), "Sell");
    
    // Test OrderType display
    assert_eq!(OrderType::Market.to_string(), "Market");
    assert_eq!(OrderType::Limit.to_string(), "Limit");
    assert_eq!(OrderType::StopMarket.to_string(), "StopMarket");
    assert_eq!(OrderType::StopLimit.to_string(), "StopLimit");
    assert_eq!(OrderType::TakeProfitMarket.to_string(), "TakeProfitMarket");
    assert_eq!(OrderType::TakeProfitLimit.to_string(), "TakeProfitLimit");
    
    // Test TimeInForce display
    assert_eq!(TimeInForce::GoodTillCancel.to_string(), "GoodTillCancel");
    assert_eq!(TimeInForce::ImmediateOrCancel.to_string(), "ImmediateOrCancel");
    assert_eq!(TimeInForce::FillOrKill.to_string(), "FillOrKill");
    assert_eq!(TimeInForce::GoodTillDate.to_string(), "GoodTillDate");
    
    // Test OrderStatus display
    assert_eq!(OrderStatus::Created.to_string(), "Created");
    assert_eq!(OrderStatus::Submitted.to_string(), "Submitted");
    assert_eq!(OrderStatus::PartiallyFilled.to_string(), "PartiallyFilled");
    assert_eq!(OrderStatus::Filled.to_string(), "Filled");
    assert_eq!(OrderStatus::Cancelled.to_string(), "Cancelled");
    assert_eq!(OrderStatus::Rejected.to_string(), "Rejected");
    assert_eq!(OrderStatus::Expired.to_string(), "Expired");
}