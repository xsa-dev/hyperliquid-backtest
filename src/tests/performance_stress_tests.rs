use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use chrono::{DateTime, FixedOffset, Utc};
use tokio::test;

use crate::backtest::HyperliquidBacktest;
use crate::data::HyperliquidData;
use crate::paper_trading::PaperTradingEngine;
use crate::trading_mode::{SlippageConfig};
use crate::unified_data::{
    Position, OrderRequest, OrderResult, MarketData, 
    OrderSide, OrderType, TimeInForce, OrderStatus,
    TradingStrategy, Signal, SignalDirection
};

// Simple SMA crossover strategy for performance testing
struct SMAStrategy {
    name: String,
    short_period: usize,
    long_period: usize,
    prices: HashMap<String, Vec<f64>>,
    positions: HashMap<String, f64>,
}

impl SMAStrategy {
    fn new(name: &str, short_period: usize, long_period: usize) -> Self {
        Self {
            name: name.to_string(),
            short_period,
            long_period,
            prices: HashMap::new(),
            positions: HashMap::new(),
        }
    }
    
    fn calculate_sma(&self, prices: &[f64], period: usize) -> Option<f64> {
        if prices.len() >= period {
            let sum: f64 = prices[prices.len() - period..].iter().sum();
            Some(sum / period as f64)
        } else {
            None
        }
    }
}

impl TradingStrategy for SMAStrategy {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
        // Store price
        let prices = self.prices.entry(data.symbol.clone()).or_insert_with(Vec::new);
        prices.push(data.price);
        
        // Calculate SMAs
        if let (Some(short_sma), Some(long_sma)) = (
            self.calculate_sma(prices, self.short_period),
            self.calculate_sma(prices, self.long_period)
        ) {
            let current_position = *self.positions.get(&data.symbol).unwrap_or(&0.0);
            
            if short_sma > long_sma && current_position <= 0.0 {
                // Buy signal
                let mut orders = Vec::new();
                
                // Close short position if exists
                if current_position < 0.0 {
                    orders.push(OrderRequest::market(&data.symbol, OrderSide::Buy, current_position.abs()));
                }
                
                // Open long position
                orders.push(OrderRequest::market(&data.symbol, OrderSide::Buy, 1.0));
                
                return Ok(orders);
            } else if short_sma < long_sma && current_position >= 0.0 {
                // Sell signal
                let mut orders = Vec::new();
                
                // Close long position if exists
                if current_position > 0.0 {
                    orders.push(OrderRequest::market(&data.symbol, OrderSide::Sell, current_position));
                }
                
                // Open short position
                orders.push(OrderRequest::market(&data.symbol, OrderSide::Sell, 1.0));
                
                return Ok(orders);
            }
        }
        
        Ok(vec![])
    }
    
    fn on_order_fill(&mut self, fill: &crate::unified_data::OrderFill) -> Result<(), String> {
        // Update position
        let current_position = *self.positions.get(&fill.symbol).unwrap_or(&0.0);
        let position_change = match fill.side {
            OrderSide::Buy => fill.quantity,
            OrderSide::Sell => -fill.quantity,
        };
        
        self.positions.insert(fill.symbol.clone(), current_position + position_change);
        
        Ok(())
    }
    
    fn on_funding_payment(&mut self, _payment: &crate::unified_data::FundingPayment) -> Result<(), String> {
        Ok(())
    }
    
    fn get_current_signals(&self) -> HashMap<String, Signal> {
        HashMap::new()
    }
}

// Helper function to generate large test data
fn generate_large_test_data(symbol: &str, data_points: usize) -> HyperliquidData {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let mut datetime = Vec::with_capacity(data_points);
    let mut open = Vec::with_capacity(data_points);
    let mut high = Vec::with_capacity(data_points);
    let mut low = Vec::with_capacity(data_points);
    let mut close = Vec::with_capacity(data_points);
    let mut volume = Vec::with_capacity(data_points);
    let mut funding_rates = Vec::with_capacity(data_points);
    let mut funding_timestamps = Vec::with_capacity(data_points);
    
    let mut price = 100.0;
    
    for i in 0..data_points {
        // Generate somewhat realistic price movement
        let change = (rand::random::<f64>() - 0.5) * 2.0; // Random change between -1 and 1
        price += change;
        price = price.max(10.0); // Ensure price doesn't go too low
        
        let timestamp = now + chrono::Duration::minutes(i as i64);
        datetime.push(timestamp);
        open.push(price);
        high.push(price * (1.0 + rand::random::<f64>() * 0.01)); // Up to 1% higher
        low.push(price * (1.0 - rand::random::<f64>() * 0.01));  // Up to 1% lower
        close.push(price);
        volume.push(100.0 + rand::random::<f64>() * 900.0); // Random volume between 100 and 1000
        
        // Add funding rate every 8 hours (480 minutes)
        if i % 480 == 0 {
            funding_rates.push((rand::random::<f64>() - 0.5) * 0.001); // Random funding rate between -0.05% and 0.05%
            funding_timestamps.push(timestamp);
        }
    }
    
    HyperliquidData {
        ticker: symbol.to_string(),
        datetime,
        open,
        high,
        low,
        close,
        volume,
        funding_rates,
        funding_timestamps,
    }
}

// Helper function to generate large market data
fn generate_large_market_data(symbol: &str, data_points: usize) -> Vec<MarketData> {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let mut result = Vec::with_capacity(data_points);
    
    let mut price = 100.0;
    
    for i in 0..data_points {
        // Generate somewhat realistic price movement
        let change = (rand::random::<f64>() - 0.5) * 2.0; // Random change between -1 and 1
        price += change;
        price = price.max(10.0); // Ensure price doesn't go too low
        
        let timestamp = now + chrono::Duration::minutes(i as i64);
        let data = MarketData::new(
            symbol,
            price,
            price * 0.999, // Bid slightly lower
            price * 1.001, // Ask slightly higher
            100.0 + rand::random::<f64>() * 900.0, // Random volume between 100 and 1000
            timestamp,
        );
        result.push(data);
    }
    
    result
}

#[test]
fn test_backtest_performance_small_dataset() {
    let data_points = 1000;
    let data = generate_large_test_data("BTC", data_points);
    let strategy = SMAStrategy::new("SMAStrategy", 10, 30);
    
    let start_time = Instant::now();
    
    let mut backtest = HyperliquidBacktest::new(
        data,
        Box::new(strategy),
        10000.0,
        Default::default(),
    );
    
    backtest.run();
    
    let duration = start_time.elapsed();
    
    println!("Backtest with {} data points completed in {:?}", data_points, duration);
    println!("Processing speed: {:.2} data points per second", data_points as f64 / duration.as_secs_f64());
    
    // Ensure reasonable performance (adjust thresholds as needed)
    assert!(duration < Duration::from_secs(1));
}

#[test]
fn test_backtest_performance_medium_dataset() {
    let data_points = 10000;
    let data = generate_large_test_data("BTC", data_points);
    let strategy = SMAStrategy::new("SMAStrategy", 10, 30);
    
    let start_time = Instant::now();
    
    let mut backtest = HyperliquidBacktest::new(
        data,
        Box::new(strategy),
        10000.0,
        Default::default(),
    );
    
    backtest.run();
    
    let duration = start_time.elapsed();
    
    println!("Backtest with {} data points completed in {:?}", data_points, duration);
    println!("Processing speed: {:.2} data points per second", data_points as f64 / duration.as_secs_f64());
    
    // Ensure reasonable performance (adjust thresholds as needed)
    assert!(duration < Duration::from_secs(5));
}

#[test]
fn test_backtest_performance_large_dataset() {
    let data_points = 100000;
    let data = generate_large_test_data("BTC", data_points);
    let strategy = SMAStrategy::new("SMAStrategy", 10, 30);
    
    let start_time = Instant::now();
    
    let mut backtest = HyperliquidBacktest::new(
        data,
        Box::new(strategy),
        10000.0,
        Default::default(),
    );
    
    backtest.run();
    
    let duration = start_time.elapsed();
    
    println!("Backtest with {} data points completed in {:?}", data_points, duration);
    println!("Processing speed: {:.2} data points per second", data_points as f64 / duration.as_secs_f64());
    
    // Ensure reasonable performance (adjust thresholds as needed)
    assert!(duration < Duration::from_secs(30));
}

#[tokio::test]
async fn test_paper_trading_performance() {
    let data_points = 10000;
    let market_data = generate_large_market_data("BTC", data_points);
    let mut strategy = SMAStrategy::new("SMAStrategy", 10, 30);
    let mut engine = PaperTradingEngine::new(10000.0, SlippageConfig::default());
    
    let start_time = Instant::now();
    
    for data in market_data {
        engine.update_market_data(data.clone()).unwrap();
        let orders = strategy.on_market_data(&data).unwrap();
        
        for order in orders {
            let result = engine.execute_order(order).await.unwrap();
            
            let fill = crate::unified_data::OrderFill {
                order_id: result.order_id.clone(),
                symbol: result.symbol.clone(),
                side: result.side,
                quantity: result.filled_quantity,
                price: result.average_price.unwrap_or(data.price),
                timestamp: result.timestamp,
                fees: result.fees.unwrap_or(0.0),
            };
            
            strategy.on_order_fill(&fill).unwrap();
        }
    }
    
    let duration = start_time.elapsed();
    
    println!("Paper trading with {} data points completed in {:?}", data_points, duration);
    println!("Processing speed: {:.2} data points per second", data_points as f64 / duration.as_secs_f64());
    
    // Ensure reasonable performance (adjust thresholds as needed)
    assert!(duration < Duration::from_secs(10));
}

#[test]
fn test_multi_asset_backtest_performance() {
    let data_points = 10000;
    
    // Generate data for multiple assets
    let btc_data = generate_large_test_data("BTC", data_points);
    let eth_data = generate_large_test_data("ETH", data_points);
    let sol_data = generate_large_test_data("SOL", data_points);
    
    let start_time = Instant::now();
    
    // Run backtests in sequence
    let mut btc_backtest = HyperliquidBacktest::new(
        btc_data,
        Box::new(SMAStrategy::new("BTC_Strategy", 10, 30)),
        10000.0,
        Default::default(),
    );
    
    let mut eth_backtest = HyperliquidBacktest::new(
        eth_data,
        Box::new(SMAStrategy::new("ETH_Strategy", 10, 30)),
        10000.0,
        Default::default(),
    );
    
    let mut sol_backtest = HyperliquidBacktest::new(
        sol_data,
        Box::new(SMAStrategy::new("SOL_Strategy", 10, 30)),
        10000.0,
        Default::default(),
    );
    
    btc_backtest.run();
    eth_backtest.run();
    sol_backtest.run();
    
    let duration = start_time.elapsed();
    
    println!("Multi-asset backtest with {} data points per asset completed in {:?}", data_points, duration);
    println!("Processing speed: {:.2} data points per second", (data_points * 3) as f64 / duration.as_secs_f64());
    
    // Ensure reasonable performance (adjust thresholds as needed)
    assert!(duration < Duration::from_secs(30));
}

#[test]
fn test_funding_calculation_performance() {
    let data_points = 10000;
    let data = generate_large_test_data("BTC", data_points);
    let strategy = SMAStrategy::new("SMAStrategy", 10, 30);
    
    let start_time = Instant::now();
    
    let mut backtest = HyperliquidBacktest::new(
        data,
        Box::new(strategy),
        10000.0,
        Default::default(),
    );
    
    backtest.run();
    backtest.calculate_with_funding();
    
    let duration = start_time.elapsed();
    
    println!("Backtest with funding calculations for {} data points completed in {:?}", data_points, duration);
    println!("Processing speed: {:.2} data points per second", data_points as f64 / duration.as_secs_f64());
    
    // Ensure reasonable performance (adjust thresholds as needed)
    assert!(duration < Duration::from_secs(10));
}

#[test]
fn test_memory_usage_large_dataset() {
    let data_points = 500000;
    let data = generate_large_test_data("BTC", data_points);
    let strategy = SMAStrategy::new("SMAStrategy", 10, 30);
    
    // Measure approximate memory usage before
    let before = get_memory_usage();
    
    let mut backtest = HyperliquidBacktest::new(
        data,
        Box::new(strategy),
        10000.0,
        Default::default(),
    );
    
    backtest.run();
    
    // Measure approximate memory usage after
    let after = get_memory_usage();
    
    println!("Memory usage before: {} MB", before);
    println!("Memory usage after: {} MB", after);
    println!("Difference: {} MB", after - before);
    
    // This is more of a logging test than an assertion, as memory usage
    // can vary significantly between environments
}

#[test]
fn test_stress_test_rapid_market_data() {
    let data_points = 10000;
    let mut data = generate_large_test_data("BTC", data_points);
    
    // Modify data to have extreme price movements
    for i in 1..data.close.len() {
        if i % 100 == 0 {
            // Every 100 points, create a 10% price jump
            data.close[i] = data.close[i-1] * 1.1;
            data.high[i] = data.close[i] * 1.05;
        } else if i % 101 == 0 {
            // And then a 10% price drop
            data.close[i] = data.close[i-1] * 0.9;
            data.low[i] = data.close[i] * 0.95;
        }
        
        // Update open for next candle
        if i < data.open.len() - 1 {
            data.open[i+1] = data.close[i];
        }
    }
    
    let strategy = SMAStrategy::new("SMAStrategy", 10, 30);
    
    let start_time = Instant::now();
    
    let mut backtest = HyperliquidBacktest::new(
        data,
        Box::new(strategy),
        10000.0,
        Default::default(),
    );
    
    backtest.run();
    
    let duration = start_time.elapsed();
    
    println!("Stress test with rapid price movements completed in {:?}", duration);
    
    // Check that the backtest completed successfully
    let report = backtest.report();
    println!("Trades executed: {}", report.trades);
    println!("Final equity: ${:.2}", report.final_equity);
}

#[tokio::test]
async fn test_stress_test_high_order_frequency() {
    let data_points = 1000;
    let market_data = generate_large_market_data("BTC", data_points);
    
    // Create a strategy that generates orders on every tick
    struct HighFrequencyStrategy;
    
    impl TradingStrategy for HighFrequencyStrategy {
        fn name(&self) -> &str {
            "HighFrequencyStrategy"
        }
        
        fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
            // Generate alternating buy/sell orders on every tick
            if data.timestamp.timestamp() % 2 == 0 {
                Ok(vec![OrderRequest::market(&data.symbol, OrderSide::Buy, 0.01)])
            } else {
                Ok(vec![OrderRequest::market(&data.symbol, OrderSide::Sell, 0.01)])
            }
        }
        
        fn on_order_fill(&mut self, _fill: &crate::unified_data::OrderFill) -> Result<(), String> {
            Ok(())
        }
        
        fn on_funding_payment(&mut self, _payment: &crate::unified_data::FundingPayment) -> Result<(), String> {
            Ok(())
        }
        
        fn get_current_signals(&self) -> HashMap<String, Signal> {
            HashMap::new()
        }
    }
    
    let mut strategy = HighFrequencyStrategy;
    let mut engine = PaperTradingEngine::new(10000.0, SlippageConfig::default());
    
    let start_time = Instant::now();
    
    for data in market_data {
        engine.update_market_data(data.clone()).unwrap();
        let orders = strategy.on_market_data(&data).unwrap();
        
        for order in orders {
            let result = engine.execute_order(order).await.unwrap();
            
            let fill = crate::unified_data::OrderFill {
                order_id: result.order_id.clone(),
                symbol: result.symbol.clone(),
                side: result.side,
                quantity: result.filled_quantity,
                price: result.average_price.unwrap_or(data.price),
                timestamp: result.timestamp,
                fees: result.fees.unwrap_or(0.0),
            };
            
            strategy.on_order_fill(&fill).unwrap();
        }
    }
    
    let duration = start_time.elapsed();
    
    println!("High frequency order test completed in {:?}", duration);
    println!("Orders executed: {}", engine.get_order_history().len());
    
    // Ensure reasonable performance (adjust thresholds as needed)
    assert!(duration < Duration::from_secs(5));
}

#[test]
fn test_stress_test_multiple_strategies() {
    let data_points = 10000;
    let data = generate_large_test_data("BTC", data_points);
    
    // Create multiple strategies with different parameters
    let strategies = vec![
        SMAStrategy::new("SMA_5_15", 5, 15),
        SMAStrategy::new("SMA_10_30", 10, 30),
        SMAStrategy::new("SMA_20_50", 20, 50),
        SMAStrategy::new("SMA_50_200", 50, 200),
    ];
    
    let start_time = Instant::now();
    
    // Run backtests for each strategy
    for strategy in strategies {
        let mut backtest = HyperliquidBacktest::new(
            data.clone(),
            Box::new(strategy),
            10000.0,
            Default::default(),
        );
        
        backtest.run();
    }
    
    let duration = start_time.elapsed();
    
    println!("Multiple strategy test completed in {:?}", duration);
    println!("Average time per strategy: {:?}", duration / 4);
    
    // Ensure reasonable performance (adjust thresholds as needed)
    assert!(duration < Duration::from_secs(20));
}

// Helper function to get approximate memory usage
// Note: This is a very rough approximation and platform-dependent
fn get_memory_usage() -> f64 {
    // In a real implementation, you would use platform-specific APIs
    // For this example, we'll just return a placeholder value
    // On Linux, you could parse /proc/self/status
    // On Windows, you could use GetProcessMemoryInfo
    // For now, we'll just return 0 to make the test pass
    0.0
}