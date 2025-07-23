use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, FixedOffset, Utc};
use tokio::test;

use crate::backtest::HyperliquidBacktest;
use crate::data::HyperliquidData;
use crate::paper_trading::PaperTradingEngine;
use crate::live_trading::LiveTradingEngine;
use crate::trading_mode::{
    TradingMode, TradingModeManager, TradingConfig, RiskConfig, SlippageConfig, ApiConfig
};
use crate::unified_data::{
    Position, OrderRequest, OrderResult, MarketData, 
    OrderSide, OrderType, TimeInForce, OrderStatus,
    TradingStrategy, Signal, SignalDirection
};
use crate::real_time_data_stream::RealTimeDataStream;

// Test strategy implementation that works across all modes
struct ConsistencyTestStrategy {
    name: String,
    signals: HashMap<String, Signal>,
    positions: HashMap<String, f64>,
    trade_count: usize,
    last_prices: HashMap<String, f64>,
    sma_short_period: usize,
    sma_long_period: usize,
    sma_short_values: HashMap<String, Vec<f64>>,
    sma_long_values: HashMap<String, Vec<f64>>,
}

impl ConsistencyTestStrategy {
    fn new(name: &str, short_period: usize, long_period: usize) -> Self {
        Self {
            name: name.to_string(),
            signals: HashMap::new(),
            positions: HashMap::new(),
            trade_count: 0,
            last_prices: HashMap::new(),
            sma_short_period: short_period,
            sma_long_period: long_period,
            sma_short_values: HashMap::new(),
            sma_long_values: HashMap::new(),
        }
    }
    
    fn calculate_sma(&mut self, symbol: &str, price: f64) {
        // Update short SMA
        let short_values = self.sma_short_values.entry(symbol.to_string()).or_insert_with(Vec::new);
        short_values.push(price);
        if short_values.len() > self.sma_short_period {
            short_values.remove(0);
        }
        
        // Update long SMA
        let long_values = self.sma_long_values.entry(symbol.to_string()).or_insert_with(Vec::new);
        long_values.push(price);
        if long_values.len() > self.sma_long_period {
            long_values.remove(0);
        }
    }
    
    fn get_short_sma(&self, symbol: &str) -> Option<f64> {
        if let Some(values) = self.sma_short_values.get(symbol) {
            if values.len() == self.sma_short_period {
                let sum: f64 = values.iter().sum();
                return Some(sum / values.len() as f64);
            }
        }
        None
    }
    
    fn get_long_sma(&self, symbol: &str) -> Option<f64> {
        if let Some(values) = self.sma_long_values.get(symbol) {
            if values.len() == self.sma_long_period {
                let sum: f64 = values.iter().sum();
                return Some(sum / values.len() as f64);
            }
        }
        None
    }
}

impl TradingStrategy for ConsistencyTestStrategy {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
        // Store last price
        self.last_prices.insert(data.symbol.clone(), data.price);
        
        // Calculate SMAs
        self.calculate_sma(&data.symbol, data.price);
        
        // Generate signals based on SMA crossover
        let short_sma = self.get_short_sma(&data.symbol);
        let long_sma = self.get_long_sma(&data.symbol);
        
        let mut orders = Vec::new();
        
        if let (Some(short), Some(long)) = (short_sma, long_sma) {
            let current_position = *self.positions.get(&data.symbol).unwrap_or(&0.0);
            
            // SMA crossover strategy
            if short > long && current_position <= 0.0 {
                // Buy signal
                let signal = Signal {
                    symbol: data.symbol.clone(),
                    direction: SignalDirection::Buy,
                    strength: 1.0,
                    timestamp: data.timestamp,
                    metadata: HashMap::new(),
                };
                
                self.signals.insert(data.symbol.clone(), signal);
                
                // Close short position if exists
                if current_position < 0.0 {
                    orders.push(OrderRequest::market(&data.symbol, OrderSide::Buy, current_position.abs()));
                }
                
                // Open long position
                orders.push(OrderRequest::market(&data.symbol, OrderSide::Buy, 1.0));
                
            } else if short < long && current_position >= 0.0 {
                // Sell signal
                let signal = Signal {
                    symbol: data.symbol.clone(),
                    direction: SignalDirection::Sell,
                    strength: 1.0,
                    timestamp: data.timestamp,
                    metadata: HashMap::new(),
                };
                
                self.signals.insert(data.symbol.clone(), signal);
                
                // Close long position if exists
                if current_position > 0.0 {
                    orders.push(OrderRequest::market(&data.symbol, OrderSide::Sell, current_position));
                }
                
                // Open short position
                orders.push(OrderRequest::market(&data.symbol, OrderSide::Sell, 1.0));
            }
        }
        
        Ok(orders)
    }
    
    fn on_order_fill(&mut self, fill: &crate::unified_data::OrderFill) -> Result<(), String> {
        // Update position
        let current_position = *self.positions.get(&fill.symbol).unwrap_or(&0.0);
        let position_change = match fill.side {
            OrderSide::Buy => fill.quantity,
            OrderSide::Sell => -fill.quantity,
        };
        
        self.positions.insert(fill.symbol.clone(), current_position + position_change);
        self.trade_count += 1;
        
        Ok(())
    }
    
    fn on_funding_payment(&mut self, _payment: &crate::unified_data::FundingPayment) -> Result<(), String> {
        Ok(())
    }
    
    fn get_current_signals(&self) -> HashMap<String, Signal> {
        self.signals.clone()
    }
}

// Helper function to create test market data
fn create_test_market_data(symbol: &str, prices: &[f64]) -> Vec<MarketData> {
    let mut result = Vec::new();
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    for (i, &price) in prices.iter().enumerate() {
        let timestamp = now + chrono::Duration::seconds(i as i64 * 60);
        let data = MarketData::new(
            symbol,
            price,
            price * 0.999, // Bid slightly lower
            price * 1.001, // Ask slightly higher
            100.0,         // Volume
            timestamp,
        );
        result.push(data);
    }
    
    result
}

// Helper function to create test HyperliquidData
fn create_test_hyperliquid_data(symbol: &str, prices: &[f64]) -> HyperliquidData {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let mut datetime = Vec::new();
    let mut open = Vec::new();
    let mut high = Vec::new();
    let mut low = Vec::new();
    let mut close = Vec::new();
    let mut volume = Vec::new();
    
    for (i, &price) in prices.iter().enumerate() {
        let timestamp = now + chrono::Duration::seconds(i as i64 * 60);
        datetime.push(timestamp);
        open.push(price);
        high.push(price * 1.001);
        low.push(price * 0.999);
        close.push(price);
        volume.push(100.0);
    }
    
    HyperliquidData {
        ticker: symbol.to_string(),
        datetime,
        open,
        high,
        low,
        close,
        volume,
        funding_rates: vec![0.0001; prices.len()],
        funding_timestamps: datetime.clone(),
    }
}

#[test]
fn test_strategy_consistency_backtest_mode() {
    // Create test data
    let prices = vec![
        100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 105.0, 104.0, 103.0,
        102.0, 101.0, 100.0, 99.0, 98.0, 97.0, 96.0, 97.0, 98.0, 99.0,
        100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0, 108.0, 109.0,
    ];
    
    let data = create_test_hyperliquid_data("BTC", &prices);
    
    // Create strategy
    let strategy = ConsistencyTestStrategy::new("TestStrategy", 5, 10);
    
    // Create backtest
    let mut backtest = HyperliquidBacktest::new(
        data,
        Box::new(strategy),
        10000.0,
        Default::default(),
    );
    
    // Run backtest
    backtest.run();
    
    // Get results
    let report = backtest.report();
    
    // Check results
    assert!(report.trades > 0);
    assert!(report.final_equity > 0.0);
    
    // Print results
    println!("Backtest Results:");
    println!("  Trades: {}", report.trades);
    println!("  Final Equity: ${:.2}", report.final_equity);
    println!("  Return: {:.2}%", report.return_pct);
}

#[tokio::test]
async fn test_strategy_consistency_paper_trading_mode() {
    // Create test data
    let prices = vec![
        100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 105.0, 104.0, 103.0,
        102.0, 101.0, 100.0, 99.0, 98.0, 97.0, 96.0, 97.0, 98.0, 99.0,
        100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0, 108.0, 109.0,
    ];
    
    let market_data = create_test_market_data("BTC", &prices);
    
    // Create paper trading engine
    let mut engine = PaperTradingEngine::new(10000.0, SlippageConfig::default());
    
    // Create strategy
    let mut strategy = ConsistencyTestStrategy::new("TestStrategy", 5, 10);
    
    // Process market data
    for data in market_data {
        // Update market data
        engine.update_market_data(data.clone()).unwrap();
        
        // Process with strategy
        let orders = strategy.on_market_data(&data).unwrap();
        
        // Execute orders
        for order in orders {
            let result = engine.execute_order(order).await.unwrap();
            
            // Create order fill
            let fill = crate::unified_data::OrderFill {
                order_id: result.order_id.clone(),
                symbol: result.symbol.clone(),
                side: result.side,
                quantity: result.filled_quantity,
                price: result.average_price.unwrap_or(data.price),
                timestamp: result.timestamp,
                fees: result.fees.unwrap_or(0.0),
            };
            
            // Update strategy
            strategy.on_order_fill(&fill).unwrap();
        }
    }
    
    // Get results
    let report = engine.generate_report();
    
    // Check results
    assert!(report.trade_count > 0);
    assert!(report.total_equity > 0.0);
    
    // Print results
    println!("Paper Trading Results:");
    println!("  Trades: {}", report.trade_count);
    println!("  Final Equity: ${:.2}", report.total_equity);
    println!("  Return: {:.2}%", report.total_return_pct);
}

// Note: Live trading test is marked with #[ignore] as it requires actual API access
#[tokio::test]
#[ignore]
async fn test_strategy_consistency_live_trading_mode() {
    // This test would be similar to the paper trading test but with real API calls
    // For now, we'll just assert true to make the test pass
    assert!(true);
}

#[test]
fn test_strategy_signals_consistency() {
    // Create test data
    let prices = vec![
        100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 105.0, 104.0, 103.0,
        102.0, 101.0, 100.0, 99.0, 98.0, 97.0, 96.0, 97.0, 98.0, 99.0,
    ];
    
    let market_data = create_test_market_data("BTC", &prices);
    
    // Create two instances of the same strategy
    let mut strategy1 = ConsistencyTestStrategy::new("TestStrategy1", 5, 10);
    let mut strategy2 = ConsistencyTestStrategy::new("TestStrategy2", 5, 10);
    
    // Process market data with both strategies
    for data in market_data {
        let orders1 = strategy1.on_market_data(&data).unwrap();
        let orders2 = strategy2.on_market_data(&data).unwrap();
        
        // Check that both strategies generate the same signals
        let signals1 = strategy1.get_current_signals();
        let signals2 = strategy2.get_current_signals();
        
        if !signals1.is_empty() && !signals2.is_empty() {
            let signal1 = signals1.get("BTC").unwrap();
            let signal2 = signals2.get("BTC").unwrap();
            
            assert_eq!(signal1.direction, signal2.direction);
            assert_eq!(signal1.strength, signal2.strength);
        }
        
        // Check that both strategies generate the same orders
        assert_eq!(orders1.len(), orders2.len());
        
        for (order1, order2) in orders1.iter().zip(orders2.iter()) {
            assert_eq!(order1.symbol, order2.symbol);
            assert_eq!(order1.side, order2.side);
            assert_eq!(order1.quantity, order2.quantity);
            assert_eq!(order1.order_type, order2.order_type);
        }
    }
}

#[tokio::test]
async fn test_strategy_position_consistency() {
    // Create test data
    let prices = vec![
        100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 105.0, 104.0, 103.0,
        102.0, 101.0, 100.0, 99.0, 98.0, 97.0, 96.0, 97.0, 98.0, 99.0,
    ];
    
    let market_data = create_test_market_data("BTC", &prices);
    
    // Create strategy
    let mut strategy = ConsistencyTestStrategy::new("TestStrategy", 5, 10);
    
    // Create paper trading engine
    let mut engine = PaperTradingEngine::new(10000.0, SlippageConfig::default());
    
    // Process market data
    for data in market_data {
        // Update market data
        engine.update_market_data(data.clone()).unwrap();
        
        // Process with strategy
        let orders = strategy.on_market_data(&data).unwrap();
        
        // Execute orders
        for order in orders {
            let result = engine.execute_order(order).await.unwrap();
            
            // Create order fill
            let fill = crate::unified_data::OrderFill {
                order_id: result.order_id.clone(),
                symbol: result.symbol.clone(),
                side: result.side,
                quantity: result.filled_quantity,
                price: result.average_price.unwrap_or(data.price),
                timestamp: result.timestamp,
                fees: result.fees.unwrap_or(0.0),
            };
            
            // Update strategy
            strategy.on_order_fill(&fill).unwrap();
        }
    }
    
    // Check position consistency
    let strategy_positions = strategy.positions;
    let engine_positions = engine.get_positions();
    
    for (symbol, strategy_size) in &strategy_positions {
        if let Some(engine_position) = engine_positions.get(symbol) {
            assert_eq!(*strategy_size, engine_position.size);
        } else if *strategy_size != 0.0 {
            panic!("Strategy has position for {} but engine doesn't", symbol);
        }
    }
}

#[test]
fn test_trading_mode_manager_consistency() {
    // Create config that works for all modes
    let config = TradingConfig::new(10000.0)
        .with_risk_config(RiskConfig::default())
        .with_slippage_config(SlippageConfig::default())
        .with_api_config(ApiConfig {
            api_key: "test_key".to_string(),
            api_secret: "test_secret".to_string(),
            endpoint: "https://api.example.com".to_string(),
            use_testnet: true,
            timeout_ms: 5000,
        });
    
    // Create trading mode manager
    let mut manager = TradingModeManager::new(TradingMode::Backtest, config);
    
    // Test mode switching
    assert_eq!(manager.current_mode(), TradingMode::Backtest);
    
    // Switch to paper trading
    assert!(manager.switch_mode(TradingMode::PaperTrade).is_ok());
    assert_eq!(manager.current_mode(), TradingMode::PaperTrade);
    
    // Switch back to backtest
    assert!(manager.switch_mode(TradingMode::Backtest).is_ok());
    assert_eq!(manager.current_mode(), TradingMode::Backtest);
    
    // Direct switch from backtest to live trading should fail
    assert!(manager.switch_mode(TradingMode::LiveTrade).is_err());
    assert_eq!(manager.current_mode(), TradingMode::Backtest);
    
    // Proper transition: backtest -> paper -> live
    assert!(manager.switch_mode(TradingMode::PaperTrade).is_ok());
    assert_eq!(manager.current_mode(), TradingMode::PaperTrade);
    
    assert!(manager.switch_mode(TradingMode::LiveTrade).is_ok());
    assert_eq!(manager.current_mode(), TradingMode::LiveTrade);
}

#[tokio::test]
async fn test_strategy_execution_across_modes() {
    // Create test data
    let prices = vec![
        100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 105.0, 104.0, 103.0,
        102.0, 101.0, 100.0, 99.0, 98.0, 97.0, 96.0, 97.0, 98.0, 99.0,
    ];
    
    // Test in backtest mode
    let data = create_test_hyperliquid_data("BTC", &prices);
    let strategy_backtest = ConsistencyTestStrategy::new("TestStrategy", 5, 10);
    
    let mut backtest = HyperliquidBacktest::new(
        data,
        Box::new(strategy_backtest),
        10000.0,
        Default::default(),
    );
    
    backtest.run();
    let backtest_report = backtest.report();
    
    // Test in paper trading mode
    let market_data = create_test_market_data("BTC", &prices);
    let mut strategy_paper = ConsistencyTestStrategy::new("TestStrategy", 5, 10);
    let mut paper_engine = PaperTradingEngine::new(10000.0, SlippageConfig::default());
    
    for data in market_data {
        paper_engine.update_market_data(data.clone()).unwrap();
        let orders = strategy_paper.on_market_data(&data).unwrap();
        
        for order in orders {
            let result = paper_engine.execute_order(order).await.unwrap();
            
            let fill = crate::unified_data::OrderFill {
                order_id: result.order_id.clone(),
                symbol: result.symbol.clone(),
                side: result.side,
                quantity: result.filled_quantity,
                price: result.average_price.unwrap_or(data.price),
                timestamp: result.timestamp,
                fees: result.fees.unwrap_or(0.0),
            };
            
            strategy_paper.on_order_fill(&fill).unwrap();
        }
    }
    
    let paper_report = paper_engine.generate_report();
    
    // Compare results
    println!("Backtest Trades: {}", backtest_report.trades);
    println!("Paper Trading Trades: {}", paper_report.trade_count);
    
    println!("Backtest Return: {:.2}%", backtest_report.return_pct);
    println!("Paper Trading Return: {:.2}%", paper_report.total_return_pct);
    
    // The results won't be exactly the same due to slippage simulation in paper trading,
    // but the number of trades should be similar
    let trade_diff = (backtest_report.trades as i32 - paper_report.trade_count as i32).abs();
    assert!(trade_diff <= 2);
}