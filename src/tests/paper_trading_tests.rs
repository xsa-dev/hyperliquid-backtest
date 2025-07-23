use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, FixedOffset, Utc};
use tokio::test;

use crate::paper_trading::{
    PaperTradingEngine, PaperTradingError, SimulatedOrder, PaperTradingMetrics, TradeLogEntry
};
use crate::trading_mode::{SlippageConfig, TradingModeError};
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
                Ok(vec![OrderRequest::market(&data.symbol, OrderSide::Buy, 0.01)])
            } else {
                Ok(vec![OrderRequest::market(&data.symbol, OrderSide::Sell, 0.01)])
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

// Mock implementation of RealTimeDataStream for testing
struct MockRealTimeDataStream {
    market_data: HashMap<String, MarketData>,
}

impl MockRealTimeDataStream {
    fn new() -> Self {
        Self {
            market_data: HashMap::new(),
        }
    }
    
    fn add_market_data(&mut self, data: MarketData) {
        self.market_data.insert(data.symbol.clone(), data);
    }
}

impl RealTimeDataStream {
    // Mock implementation for testing
    pub fn mock() -> Self {
        Self::new().unwrap()
    }
}

#[tokio::test]
async fn test_paper_trading_engine_initialization() {
    let initial_balance = 10000.0;
    let slippage_config = SlippageConfig::default();
    
    let engine = PaperTradingEngine::new(initial_balance, slippage_config);
    
    assert_eq!(engine.get_balance(), initial_balance);
    assert!(engine.get_positions().is_empty());
    assert!(engine.get_order_history().is_empty());
    assert!(engine.get_active_orders().is_empty());
    assert!(engine.get_trade_log().is_empty());
    
    let metrics = engine.get_metrics();
    assert_eq!(metrics.initial_balance, initial_balance);
    assert_eq!(metrics.current_balance, initial_balance);
    assert_eq!(metrics.realized_pnl, 0.0);
    assert_eq!(metrics.unrealized_pnl, 0.0);
    assert_eq!(metrics.funding_pnl, 0.0);
    assert_eq!(metrics.total_fees, 0.0);
    assert_eq!(metrics.trade_count, 0);
    assert_eq!(metrics.winning_trades, 0);
    assert_eq!(metrics.losing_trades, 0);
}

#[tokio::test]
async fn test_paper_trading_market_data_update() {
    let initial_balance = 10000.0;
    let slippage_config = SlippageConfig::default();
    
    let mut engine = PaperTradingEngine::new(initial_balance, slippage_config);
    
    // Create market data
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let market_data = MarketData::new(
        "BTC",
        50000.0,
        49990.0,
        50010.0,
        100.0,
        now,
    );
    
    // Update market data
    let result = engine.update_market_data(market_data.clone());
    assert!(result.is_ok());
    
    // Add a position and update market data again
    let position = Position::new(
        "BTC",
        0.1,
        49000.0,
        50000.0,
        now,
    );
    
    engine.add_position(position).unwrap();
    
    // Update with new price
    let updated_market_data = MarketData::new(
        "BTC",
        51000.0,
        50990.0,
        51010.0,
        100.0,
        now,
    );
    
    let result = engine.update_market_data(updated_market_data);
    assert!(result.is_ok());
    
    // Check position was updated
    let positions = engine.get_positions();
    assert_eq!(positions.len(), 1);
    
    let btc_position = positions.get("BTC").unwrap();
    assert_eq!(btc_position.current_price, 51000.0);
    
    // Unrealized PnL should be (51000 - 49000) * 0.1 = 200.0
    assert_eq!(btc_position.unrealized_pnl, 200.0);
}

#[tokio::test]
async fn test_paper_trading_order_execution() {
    let initial_balance = 10000.0;
    let slippage_config = SlippageConfig::default();
    
    let mut engine = PaperTradingEngine::new(initial_balance, slippage_config);
    
    // Create market data
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let market_data = MarketData::new(
        "BTC",
        50000.0,
        49990.0,
        50010.0,
        100.0,
        now,
    );
    
    // Update market data
    engine.update_market_data(market_data.clone()).unwrap();
    
    // Create a market buy order
    let order = OrderRequest::market("BTC", OrderSide::Buy, 0.1);
    
    // Execute order
    let result = engine.execute_order(order).await;
    assert!(result.is_ok());
    
    let order_result = result.unwrap();
    assert_eq!(order_result.status, OrderStatus::Filled);
    assert_eq!(order_result.filled_quantity, 0.1);
    assert!(order_result.average_price.is_some());
    
    // Check position was created
    let positions = engine.get_positions();
    assert_eq!(positions.len(), 1);
    
    let btc_position = positions.get("BTC").unwrap();
    assert_eq!(btc_position.size, 0.1);
    
    // Check balance was reduced
    // Balance = initial - (price * quantity) - fees
    let price = order_result.average_price.unwrap();
    let fees = order_result.fees.unwrap();
    let expected_balance = initial_balance - (price * 0.1) - fees;
    
    assert_eq!(engine.get_balance(), expected_balance);
    
    // Check order history
    let order_history = engine.get_order_history();
    assert_eq!(order_history.len(), 1);
    
    // Check trade log
    let trade_log = engine.get_trade_log();
    assert_eq!(trade_log.len(), 1);
    
    // Create a market sell order
    let order = OrderRequest::market("BTC", OrderSide::Sell, 0.05);
    
    // Execute order
    let result = engine.execute_order(order).await;
    assert!(result.is_ok());
    
    // Check position was updated
    let positions = engine.get_positions();
    let btc_position = positions.get("BTC").unwrap();
    assert_eq!(btc_position.size, 0.05);
    
    // Check order history and trade log
    assert_eq!(engine.get_order_history().len(), 2);
    assert_eq!(engine.get_trade_log().len(), 2);
}

#[tokio::test]
async fn test_paper_trading_limit_orders() {
    let initial_balance = 10000.0;
    let slippage_config = SlippageConfig::default();
    
    let mut engine = PaperTradingEngine::new(initial_balance, slippage_config);
    
    // Create market data
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let market_data = MarketData::new(
        "BTC",
        50000.0,
        49990.0,
        50010.0,
        100.0,
        now,
    );
    
    // Update market data
    engine.update_market_data(market_data.clone()).unwrap();
    
    // Create a limit buy order below current price
    let order = OrderRequest::limit("BTC", OrderSide::Buy, 0.1, 49000.0);
    
    // Execute order
    let result = engine.execute_order(order).await;
    assert!(result.is_ok());
    
    let order_result = result.unwrap();
    assert_eq!(order_result.status, OrderStatus::Submitted);
    
    // Check active orders
    let active_orders = engine.get_active_orders();
    assert_eq!(active_orders.len(), 1);
    
    // Update market data with lower price that should trigger the limit order
    let updated_market_data = MarketData::new(
        "BTC",
        48900.0,
        48890.0,
        48910.0,
        100.0,
        now,
    );
    
    engine.update_market_data(updated_market_data).unwrap();
    
    // Check active orders (should be empty now)
    let active_orders = engine.get_active_orders();
    assert_eq!(active_orders.len(), 0);
    
    // Check order history
    let order_history = engine.get_order_history();
    assert_eq!(order_history.len(), 1);
    assert_eq!(order_history[0].result.status, OrderStatus::Filled);
    
    // Check position was created
    let positions = engine.get_positions();
    assert_eq!(positions.len(), 1);
    
    let btc_position = positions.get("BTC").unwrap();
    assert_eq!(btc_position.size, 0.1);
    assert_eq!(btc_position.entry_price, 49000.0); // Should be filled at limit price
}

#[tokio::test]
async fn test_paper_trading_slippage_model() {
    // Create custom slippage config with high slippage
    let slippage_config = SlippageConfig {
        base_slippage_pct: 0.01, // 1% base slippage
        volume_impact_factor: 0.5,
        volatility_impact_factor: 0.2,
        random_slippage_max_pct: 0.005,
        simulated_latency_ms: 100,
    };
    
    let mut engine = PaperTradingEngine::new(10000.0, slippage_config);
    
    // Create market data
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let market_data = MarketData::new(
        "BTC",
        50000.0,
        49990.0,
        50010.0,
        100.0,
        now,
    );
    
    // Update market data
    engine.update_market_data(market_data.clone()).unwrap();
    
    // Create a market buy order
    let order = OrderRequest::market("BTC", OrderSide::Buy, 0.1);
    
    // Execute order
    let result = engine.execute_order(order).await;
    assert!(result.is_ok());
    
    let order_result = result.unwrap();
    
    // Check that execution price includes slippage (should be higher than market price for buy)
    let execution_price = order_result.average_price.unwrap();
    assert!(execution_price > market_data.price);
    
    // Create a market sell order
    let order = OrderRequest::market("BTC", OrderSide::Sell, 0.05);
    
    // Execute order
    let result = engine.execute_order(order).await;
    assert!(result.is_ok());
    
    let order_result = result.unwrap();
    
    // Check that execution price includes slippage (should be lower than market price for sell)
    let execution_price = order_result.average_price.unwrap();
    assert!(execution_price < market_data.price);
}

#[tokio::test]
async fn test_paper_trading_funding_payments() {
    let mut engine = PaperTradingEngine::new(10000.0, SlippageConfig::default());
    
    // Create market data
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let market_data = MarketData::new(
        "BTC",
        50000.0,
        49990.0,
        50010.0,
        100.0,
        now,
    );
    
    // Update market data
    engine.update_market_data(market_data.clone()).unwrap();
    
    // Create a position
    let position = Position::new(
        "BTC",
        0.1,
        49000.0,
        50000.0,
        now,
    );
    
    engine.add_position(position).unwrap();
    
    // Apply positive funding payment
    let result = engine.apply_funding_payment("BTC", 10.0);
    assert!(result.is_ok());
    
    // Check funding PnL was updated
    let positions = engine.get_positions();
    let btc_position = positions.get("BTC").unwrap();
    assert_eq!(btc_position.funding_pnl, 10.0);
    
    // Check metrics
    let metrics = engine.get_metrics();
    assert_eq!(metrics.funding_pnl, 10.0);
    
    // Apply negative funding payment
    let result = engine.apply_funding_payment("BTC", -5.0);
    assert!(result.is_ok());
    
    // Check funding PnL was updated
    let positions = engine.get_positions();
    let btc_position = positions.get("BTC").unwrap();
    assert_eq!(btc_position.funding_pnl, 5.0);
    
    // Check metrics
    let metrics = engine.get_metrics();
    assert_eq!(metrics.funding_pnl, 5.0);
}

#[tokio::test]
async fn test_paper_trading_performance_report() {
    let initial_balance = 10000.0;
    let mut engine = PaperTradingEngine::new(initial_balance, SlippageConfig::default());
    
    // Create market data
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let market_data = MarketData::new(
        "BTC",
        50000.0,
        49990.0,
        50010.0,
        100.0,
        now,
    );
    
    // Update market data
    engine.update_market_data(market_data.clone()).unwrap();
    
    // Execute buy order
    let buy_order = OrderRequest::market("BTC", OrderSide::Buy, 0.1);
    engine.execute_order(buy_order).await.unwrap();
    
    // Update market data with higher price
    let updated_market_data = MarketData::new(
        "BTC",
        52000.0,
        51990.0,
        52010.0,
        100.0,
        now,
    );
    
    engine.update_market_data(updated_market_data).unwrap();
    
    // Execute sell order
    let sell_order = OrderRequest::market("BTC", OrderSide::Sell, 0.1);
    engine.execute_order(sell_order).await.unwrap();
    
    // Generate report
    let report = engine.generate_report();
    
    // Check report values
    assert_eq!(report.initial_balance, initial_balance);
    assert!(report.realized_pnl > 0.0); // Should have profit
    assert_eq!(report.unrealized_pnl, 0.0); // No open positions
    assert_eq!(report.trade_count, 2);
    assert_eq!(report.winning_trades, 1);
    assert_eq!(report.losing_trades, 0);
    
    // Check total return
    let total_return = report.total_return;
    assert!(total_return > 0.0);
    
    // Check return percentage
    let total_return_pct = report.total_return_pct;
    assert!(total_return_pct > 0.0);
}

#[tokio::test]
async fn test_paper_trading_strategy_execution() {
    let mut engine = PaperTradingEngine::new(10000.0, SlippageConfig::default());
    
    // Create mock strategy
    let strategy = Box::new(MockStrategy::new("TestStrategy", true));
    
    // Create mock real-time data stream
    let mut mock_stream = RealTimeDataStream::mock();
    
    // Add market data to stream
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let market_data = MarketData::new(
        "BTC",
        50000.0,
        49990.0,
        50010.0,
        100.0,
        now,
    );
    
    // Set up the real-time data stream
    let stream_arc = Arc::new(Mutex::new(mock_stream));
    engine.set_real_time_data(stream_arc.clone());
    
    // Update market data directly (since we can't use the mock stream in tests)
    engine.update_market_data(market_data.clone()).unwrap();
    
    // Process market data with strategy
    let result = engine.process_market_data_updates(strategy.as_ref());
    assert!(result.is_ok());
    
    // Check that orders were generated and executed
    let order_history = engine.get_order_history();
    assert!(!order_history.is_empty());
    
    // Check positions
    let positions = engine.get_positions();
    assert!(!positions.is_empty());
}

#[tokio::test]
async fn test_paper_trading_error_handling() {
    let mut engine = PaperTradingEngine::new(100.0, SlippageConfig::default()); // Small balance
    
    // Create market data
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let market_data = MarketData::new(
        "BTC",
        50000.0,
        49990.0,
        50010.0,
        100.0,
        now,
    );
    
    // Update market data
    engine.update_market_data(market_data.clone()).unwrap();
    
    // Try to execute order with insufficient balance
    let order = OrderRequest::market("BTC", OrderSide::Buy, 1.0); // 1 BTC at 50000 = 50000 USD
    
    let result = engine.execute_order(order).await;
    assert!(result.is_err());
    
    match result {
        Err(PaperTradingError::InsufficientBalance { required, available }) => {
            assert!(required > available);
        },
        _ => panic!("Expected InsufficientBalance error"),
    }
    
    // Try to get market data for non-existent symbol
    let result = engine.get_market_data("ETH");
    assert!(result.is_err());
    
    match result {
        Err(PaperTradingError::MarketDataNotAvailable(symbol)) => {
            assert_eq!(symbol, "ETH");
        },
        _ => panic!("Expected MarketDataNotAvailable error"),
    }
    
    // Try to apply funding payment to non-existent position
    let result = engine.apply_funding_payment("ETH", 10.0);
    assert!(result.is_err());
    
    match result {
        Err(PaperTradingError::PositionNotFound(symbol)) => {
            assert_eq!(symbol, "ETH");
        },
        _ => panic!("Expected PositionNotFound error"),
    }
}

#[tokio::test]
async fn test_paper_trading_multi_asset() {
    let mut engine = PaperTradingEngine::new(100000.0, SlippageConfig::default());
    
    // Create market data for multiple assets
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    let btc_data = MarketData::new(
        "BTC",
        50000.0,
        49990.0,
        50010.0,
        100.0,
        now,
    );
    
    let eth_data = MarketData::new(
        "ETH",
        3000.0,
        2990.0,
        3010.0,
        1000.0,
        now,
    );
    
    let sol_data = MarketData::new(
        "SOL",
        100.0,
        99.0,
        101.0,
        10000.0,
        now,
    );
    
    // Update market data
    engine.update_market_data(btc_data.clone()).unwrap();
    engine.update_market_data(eth_data.clone()).unwrap();
    engine.update_market_data(sol_data.clone()).unwrap();
    
    // Execute orders for multiple assets
    let btc_order = OrderRequest::market("BTC", OrderSide::Buy, 0.1);
    let eth_order = OrderRequest::market("ETH", OrderSide::Buy, 1.0);
    let sol_order = OrderRequest::market("SOL", OrderSide::Buy, 10.0);
    
    engine.execute_order(btc_order).await.unwrap();
    engine.execute_order(eth_order).await.unwrap();
    engine.execute_order(sol_order).await.unwrap();
    
    // Check positions
    let positions = engine.get_positions();
    assert_eq!(positions.len(), 3);
    assert!(positions.contains_key("BTC"));
    assert!(positions.contains_key("ETH"));
    assert!(positions.contains_key("SOL"));
    
    // Update prices
    let btc_data_updated = MarketData::new(
        "BTC",
        52000.0,
        51990.0,
        52010.0,
        100.0,
        now,
    );
    
    let eth_data_updated = MarketData::new(
        "ETH",
        3200.0,
        3190.0,
        3210.0,
        1000.0,
        now,
    );
    
    let sol_data_updated = MarketData::new(
        "SOL",
        90.0,
        89.0,
        91.0,
        10000.0,
        now,
    );
    
    engine.update_market_data(btc_data_updated).unwrap();
    engine.update_market_data(eth_data_updated).unwrap();
    engine.update_market_data(sol_data_updated).unwrap();
    
    // Check unrealized PnL
    let positions = engine.get_positions();
    
    let btc_position = positions.get("BTC").unwrap();
    let eth_position = positions.get("ETH").unwrap();
    let sol_position = positions.get("SOL").unwrap();
    
    assert!(btc_position.unrealized_pnl > 0.0); // BTC price increased
    assert!(eth_position.unrealized_pnl > 0.0); // ETH price increased
    assert!(sol_position.unrealized_pnl < 0.0); // SOL price decreased
    
    // Generate report
    let report = engine.generate_report();
    
    // Portfolio should have mixed results
    assert!(report.unrealized_pnl > 0.0); // Overall should be positive
}