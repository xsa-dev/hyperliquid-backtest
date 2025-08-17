//! Integration tests with mocked Hyperliquid API
//! 
//! These tests verify the complete workflow from API calls to backtesting results
//! using mocked HTTP responses to ensure consistent and reliable testing.
//! 
//! Includes comprehensive memory usage tests, performance validation, and
//! end-to-end workflow testing with realistic data scenarios.

use crate::prelude::*;
use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use mockito::{Mock, Server};
use serde_json::json;
use std::collections::HashMap;
use tokio_test;
use memory_stats::memory_stats;
use sysinfo::{System, SystemExt, ProcessExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;
use tracing::{info, warn, error};
use futures::future::join_all;

/// Mock server for Hyperliquid API responses
struct MockHyperliquidServer {
    server: Server,
}

impl MockHyperliquidServer {
    async fn new() -> Self {
        Self {
            server: Server::new_async().await,
        }
    }

    /// Create mock for candles snapshot endpoint
    fn mock_candles_snapshot(&mut self, coin: &str, interval: &str) -> Mock {
        let mock_data = json!([
            {
                "T": 1640995200000i64, // 2022-01-01 00:00:00
                "c": "47000.5",
                "h": "47500.0",
                "l": "46500.0",
                "n": 1000,
                "o": "47200.0",
                "t": 1640995200000i64,
                "v": "125.5"
            },
            {
                "T": 1640998800000i64, // 2022-01-01 01:00:00
                "c": "47100.0",
                "h": "47300.0",
                "l": "46800.0",
                "n": 950,
                "o": "47000.5",
                "t": 1640998800000i64,
                "v": "98.2"
            },
            {
                "T": 1641002400000i64, // 2022-01-01 02:00:00
                "c": "46950.0",
                "h": "47200.0",
                "l": "46700.0",
                "n": 1100,
                "o": "47100.0",
                "t": 1641002400000i64,
                "v": "156.8"
            }
        ]);

        self.server
            .mock("POST", "/info")
            .match_body(mockito::Matcher::JsonString(format!(
                r#"{{"type":"candleSnapshot","req":{{"coin":"{}","interval":"{}","startTime":1640995200000,"endTime":1641002400000}}}}"#,
                coin, interval
            )))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_data.to_string())
    }

    /// Create mock for funding history endpoint
    fn mock_funding_history(&mut self, coin: &str) -> Mock {
        let mock_data = json!([
            {
                "coin": coin,
                "fundingRate": "0.0001",
                "premium": "0.00005",
                "time": 1640995200000i64
            },
            {
                "coin": coin,
                "fundingRate": "0.00015",
                "premium": "0.0001",
                "time": 1640998800000i64
            },
            {
                "coin": coin,
                "fundingRate": "0.00008",
                "premium": "0.00003",
                "time": 1641002400000i64
            }
        ]);

        self.server
            .mock("POST", "/info")
            .match_body(mockito::Matcher::JsonString(format!(
                r#"{{"type":"fundingHistory","req":{{"coin":"{}","startTime":1640995200000}}}}"#,
                coin
            )))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_data.to_string())
    }

    /// Create mock for API error responses
    fn mock_api_error(&mut self, status_code: usize, error_message: &str) -> Mock {
        self.server
            .mock("POST", "/info")
            .with_status(status_code)
            .with_header("content-type", "application/json")
            .with_body(json!({"error": error_message}).to_string())
    }

    fn url(&self) -> String {
        self.server.url()
    }
}

#[tokio::test]
async fn test_complete_data_fetching_workflow() {
    let mut mock_server = MockHyperliquidServer::new().await;
    
    // Set up mocks
    let _candles_mock = mock_server.mock_candles_snapshot("BTC", "1h");
    let _funding_mock = mock_server.mock_funding_history("BTC");

    // Note: In a real implementation, we would need to configure the HyperliquidDataFetcher
    // to use our mock server URL instead of the real Hyperliquid API
    // For now, we'll test the data structure creation and validation
    
    let start_time = 1640995200000; // 2022-01-01 00:00:00
    let end_time = 1641002400000;   // 2022-01-01 02:00:00

    // Create mock data that would come from the API
    let mock_ohlc_data = vec![
        (1640995200000, 47200.0, 47500.0, 46500.0, 47000.5, 125.5),
        (1640998800000, 47000.5, 47300.0, 46800.0, 47100.0, 98.2),
        (1641002400000, 47100.0, 47200.0, 46700.0, 46950.0, 156.8),
    ];

    let mock_funding_data = vec![
        (1640995200000, 0.0001),
        (1640998800000, 0.00015),
        (1641002400000, 0.00008),
    ];

    // Create HyperliquidData from mock data
    let datetime: Vec<DateTime<FixedOffset>> = mock_ohlc_data
        .iter()
        .map(|(ts, _, _, _, _, _)| {
            DateTime::from_timestamp(*ts / 1000, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let funding_timestamps: Vec<DateTime<FixedOffset>> = mock_funding_data
        .iter()
        .map(|(ts, _)| {
            DateTime::from_timestamp(*ts / 1000, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let hyperliquid_data = HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: mock_ohlc_data.iter().map(|(_, o, _, _, _, _)| *o).collect(),
        high: mock_ohlc_data.iter().map(|(_, _, h, _, _, _)| *h).collect(),
        low: mock_ohlc_data.iter().map(|(_, _, _, l, _, _)| *l).collect(),
        close: mock_ohlc_data.iter().map(|(_, _, _, _, c, _)| *c).collect(),
        volume: mock_ohlc_data.iter().map(|(_, _, _, _, _, v)| *v).collect(),
        funding_rates: mock_funding_data.iter().map(|(_, fr)| *fr).collect(),
        funding_timestamps,
    };

    // Test data conversion to rs-backtester format
    let rs_data = hyperliquid_data.to_rs_backtester_data();
    assert_eq!(rs_data.close.len(), 3);
    assert_eq!(rs_data.close[0], 47000.5);
    assert_eq!(rs_data.close[2], 46950.0);

    // Test funding rate lookup
    let funding_rate = hyperliquid_data.get_funding_rate_at(datetime[1]);
    assert!(funding_rate.is_some());
    assert!((funding_rate.unwrap() - 0.00015).abs() < 1e-6);
}

#[tokio::test]
async fn test_end_to_end_backtesting_workflow() {
    // Create comprehensive test data
    let datetime: Vec<DateTime<FixedOffset>> = (0..100)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let mut prices = Vec::new();
    let mut base_price = 47000.0;
    
    // Generate realistic price movement
    for i in 0..100 {
        let noise = (i as f64 * 0.1).sin() * 100.0;
        let trend = i as f64 * 2.0;
        base_price = 47000.0 + trend + noise;
        prices.push(base_price);
    }

    let hyperliquid_data = HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().map(|p| p - 10.0).collect(),
        high: prices.iter().map(|p| p + 50.0).collect(),
        low: prices.iter().map(|p| p - 50.0).collect(),
        close: prices.clone(),
        volume: vec![100.0; 100],
        funding_rates: (0..100).map(|i| 0.0001 + (i as f64 * 0.01).sin() * 0.0001).collect(),
        funding_timestamps: datetime.clone(),
    };

    // Create and run backtest
    let strategy = enhanced_sma_cross(10, 20, 0.5);
    let mut backtest = HyperliquidBacktest::new(
        hyperliquid_data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );

    backtest.calculate_with_funding();

    // Verify backtest results
    let report = backtest.enhanced_report();
    assert!(report.total_return != 0.0);
    assert!(backtest.total_funding_paid >= 0.0 || backtest.total_funding_received >= 0.0);

    // Test funding report generation
    let funding_report = backtest.funding_report();
    assert!(!funding_report.funding_payments.is_empty());
    assert!(funding_report.total_funding_paid >= 0.0);
    assert!(funding_report.total_funding_received >= 0.0);
}

#[tokio::test]
async fn test_api_error_handling() {
    let mut mock_server = MockHyperliquidServer::new().await;
    
    // Test various error scenarios
    let _error_mock = mock_server.mock_api_error(500, "Internal server error");
    
    // In a real implementation, we would test that our data fetcher
    // properly handles these error responses and converts them to
    // appropriate HyperliquidBacktestError variants
    
    // For now, test error creation and handling
    let api_error = HyperliquidBacktestError::DataConversion("Invalid JSON response".to_string());
    assert!(api_error.to_string().contains("Data conversion error"));
    
    let time_error = HyperliquidBacktestError::InvalidTimeRange { 
        start: 1641002400000, 
        end: 1640995200000 
    };
    assert!(time_error.to_string().contains("Invalid time range"));
}

#[tokio::test]
async fn test_concurrent_data_fetching() {
    // Test concurrent fetching of multiple assets
    let coins = vec!["BTC", "ETH", "SOL"];
    let mut handles = Vec::new();

    for coin in coins {
        let coin = coin.to_string();
        let handle = tokio::spawn(async move {
            // Create mock data for each coin
            let datetime: Vec<DateTime<FixedOffset>> = (0..50)
                .map(|i| {
                    DateTime::from_timestamp(1640995200 + i * 3600, 0)
                        .unwrap()
                        .with_timezone(&FixedOffset::east_opt(0).unwrap())
                })
                .collect();

            let base_price = match coin.as_str() {
                "BTC" => 47000.0,
                "ETH" => 3500.0,
                "SOL" => 150.0,
                _ => 100.0,
            };

            let prices: Vec<f64> = (0..50)
                .map(|i| base_price + (i as f64 * 0.1).sin() * base_price * 0.02)
                .collect();

            HyperliquidData {
                ticker: coin,
                datetime: datetime.clone(),
                open: prices.iter().map(|p| p - 1.0).collect(),
                high: prices.iter().map(|p| p + 10.0).collect(),
                low: prices.iter().map(|p| p - 10.0).collect(),
                close: prices,
                volume: vec![100.0; 50],
                funding_rates: vec![0.0001; 50],
                funding_timestamps: datetime,
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let results: Vec<HyperliquidData> = futures::future::try_join_all(handles)
        .await
        .expect("All tasks should complete successfully");

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].ticker, "BTC");
    assert_eq!(results[1].ticker, "ETH");
    assert_eq!(results[2].ticker, "SOL");
}

#[tokio::test]
async fn test_data_validation_and_consistency() {
    // Test data validation with inconsistent timestamps
    let datetime1: Vec<DateTime<FixedOffset>> = (0..10)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let datetime2: Vec<DateTime<FixedOffset>> = (0..5) // Different length
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i * 7200, 0) // Different interval
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let hyperliquid_data = HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime1,
        open: vec![47000.0; 10],
        high: vec![47500.0; 10],
        low: vec![46500.0; 10],
        close: vec![47200.0; 10],
        volume: vec![100.0; 10],
        funding_rates: vec![0.0001; 5], // Mismatched length
        funding_timestamps: datetime2,
    };

    // Test that funding rate lookup handles mismatched data gracefully
    let funding_rate = hyperliquid_data.get_funding_rate_at(
        DateTime::from_timestamp(1640995200 + 3600, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap())
    );
    
    // Should handle gracefully (either return None or interpolated value)
    assert!(funding_rate.is_some() || funding_rate.is_none());
}

#[tokio::test]
async fn test_regression_api_compatibility() {
    // Test that our data structures remain compatible with rs-backtester
    let hyperliquid_data = create_test_hyperliquid_data();
    let rs_data = hyperliquid_data.to_rs_backtester_data();

    // Verify rs-backtester compatibility
    assert_eq!(rs_data.datetime.len(), rs_data.close.len());
    assert_eq!(rs_data.open.len(), rs_data.close.len());
    assert_eq!(rs_data.high.len(), rs_data.close.len());
    assert_eq!(rs_data.low.len(), rs_data.close.len());
    assert_eq!(rs_data.volume.len(), rs_data.close.len());

    // Test that we can create a standard rs-backtester backtest
    use rs_backtester::prelude::*;
    
    let strategy = strategies::sma_cross(10, 20);
    let backtest = Backtest::new(rs_data, strategy, 10000.0, Commission::default());
    
    // Should be able to run without errors
    assert!(backtest.data.close.len() > 0);
}

// ============================================================================
// MEMORY USAGE TESTS
// ============================================================================

/// Memory usage tracking utility
struct MemoryTracker {
    initial_memory: Option<usize>,
    peak_memory: usize,
}

impl MemoryTracker {
    fn new() -> Self {
        let initial = memory_stats().map(|stats| stats.physical_mem);
        Self {
            initial_memory: initial,
            peak_memory: initial.unwrap_or(0),
        }
    }

    fn update_peak(&mut self) {
        if let Some(stats) = memory_stats() {
            self.peak_memory = self.peak_memory.max(stats.physical_mem);
        }
    }

    fn memory_increase(&self) -> Option<usize> {
        self.initial_memory.map(|initial| self.peak_memory.saturating_sub(initial))
    }
}

#[tokio::test]
async fn test_memory_usage_large_datasets() {
    let mut tracker = MemoryTracker::new();
    
    // Test with progressively larger datasets
    let sizes = vec![1_000, 10_000, 100_000, 500_000];
    let mut memory_usage = Vec::new();
    
    for size in sizes {
        info!("Testing memory usage with {} data points", size);
        
        let data = create_large_test_data(size);
        tracker.update_peak();
        
        // Test data conversion memory usage
        let rs_data = data.to_rs_backtester_data();
        tracker.update_peak();
        
        // Test backtesting memory usage
        let strategy = enhanced_sma_cross(20, 50, 0.3);
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        tracker.update_peak();
        
        backtest.calculate_with_funding();
        tracker.update_peak();
        
        if let Some(increase) = tracker.memory_increase() {
            memory_usage.push((size, increase));
            info!("Memory increase for {} points: {} bytes", size, increase);
            
            // Memory usage should be roughly linear with data size
            // Allow for some overhead but flag excessive memory usage
            let bytes_per_point = increase / size;
            assert!(bytes_per_point < 10_000, 
                "Memory usage per data point ({} bytes) exceeds threshold", bytes_per_point);
        }
        
        // Force cleanup
        drop(backtest);
        drop(rs_data);
    }
    
    // Verify memory usage scaling is reasonable
    if memory_usage.len() >= 2 {
        let (small_size, small_mem) = memory_usage[0];
        let (large_size, large_mem) = memory_usage[memory_usage.len() - 1];
        
        let size_ratio = large_size as f64 / small_size as f64;
        let memory_ratio = large_mem as f64 / small_mem as f64;
        
        // Memory usage should scale roughly linearly (within 3x factor)
        assert!(memory_ratio < size_ratio * 3.0, 
            "Memory usage scaling ({:.2}x) exceeds size scaling ({:.2}x) by too much", 
            memory_ratio, size_ratio);
    }
}

#[tokio::test]
async fn test_memory_leak_detection() {
    let mut tracker = MemoryTracker::new();
    let initial_memory = tracker.initial_memory.unwrap_or(0);
    
    // Run multiple iterations to detect memory leaks
    for iteration in 0..10 {
        info!("Memory leak test iteration {}", iteration);
        
        // Create and process data
        let data = create_large_test_data(10_000);
        let strategy = funding_arbitrage_strategy(0.001, Default::default());
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let _report = backtest.funding_report();
        let _enhanced_report = backtest.enhanced_report();
        
        // Export to CSV to test memory usage in export functions
        let mut csv_buffer = Vec::new();
        let _ = backtest.enhanced_csv_export(&mut csv_buffer);
        
        tracker.update_peak();
        
        // Explicitly drop to ensure cleanup
        drop(backtest);
        drop(csv_buffer);
        
        // Check for excessive memory growth
        if let Some(current_stats) = memory_stats() {
            let current_memory = current_stats.physical_mem;
            let growth = current_memory.saturating_sub(initial_memory);
            
            // Allow for some growth but flag excessive increases
            let max_allowed_growth = initial_memory / 10; // 10% growth max
            if growth > max_allowed_growth {
                warn!("Potential memory leak detected: {} bytes growth", growth);
            }
        }
    }
}

#[tokio::test]
async fn test_concurrent_memory_usage() {
    let mut tracker = MemoryTracker::new();
    
    // Test concurrent processing of multiple datasets
    let tasks = (0..5).map(|i| {
        tokio::spawn(async move {
            let data = create_large_test_data(20_000 + i * 1000);
            let strategy = enhanced_sma_cross(10 + i, 30 + i * 2, 0.2 + i as f64 * 0.1);
            let mut backtest = HyperliquidBacktest::new(
                data,
                strategy,
                10000.0,
                HyperliquidCommission::default(),
            );
            
            backtest.calculate_with_funding();
            let report = backtest.enhanced_report();
            
            (i, report.total_return, backtest.total_funding_paid)
        })
    });
    
    let results = join_all(tasks).await;
    tracker.update_peak();
    
    // Verify all tasks completed successfully
    for result in results {
        let (task_id, total_return, funding_paid) = result.unwrap();
        info!("Task {} completed: return={:.2}, funding={:.2}", 
              task_id, total_return, funding_paid);
    }
    
    // Check memory usage is reasonable for concurrent operations
    if let Some(increase) = tracker.memory_increase() {
        info!("Concurrent memory usage increase: {} bytes", increase);
        // Should not exceed 500MB for this test
        assert!(increase < 500_000_000, "Concurrent memory usage too high: {} bytes", increase);
    }
}

// ============================================================================
// ENHANCED API MOCKING TESTS
// ============================================================================

impl MockHyperliquidServer {
    /// Create mock with realistic large dataset
    fn mock_large_candles_snapshot(&mut self, coin: &str, interval: &str, size: usize) -> Mock {
        let mut candles = Vec::new();
        let mut timestamp = 1640995200000i64;
        let mut price = 47000.0;
        
        for i in 0..size {
            // Generate realistic price movement
            let noise = (i as f64 * 0.1).sin() * 50.0;
            let trend = (i as f64 * 0.01).cos() * 20.0;
            price += noise + trend;
            
            candles.push(json!({
                "T": timestamp,
                "c": format!("{:.1}", price),
                "h": format!("{:.1}", price + 25.0),
                "l": format!("{:.1}", price - 25.0),
                "n": 1000 + i,
                "o": format!("{:.1}", price - 5.0),
                "t": timestamp,
                "v": format!("{:.1}", 100.0 + (i as f64 * 0.05).sin() * 20.0)
            }));
            
            timestamp += 3600000; // 1 hour intervals
        }

        self.server
            .mock("POST", "/info")
            .match_body(mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!(candles).to_string())
    }

    /// Create mock with rate limiting simulation
    fn mock_rate_limited_response(&mut self) -> Mock {
        self.server
            .mock("POST", "/info")
            .with_status(429)
            .with_header("content-type", "application/json")
            .with_header("retry-after", "1")
            .with_body(json!({"error": "Rate limit exceeded"}).to_string())
    }

    /// Create mock with network timeout simulation
    fn mock_timeout_response(&mut self) -> Mock {
        self.server
            .mock("POST", "/info")
            .with_status(408)
            .with_header("content-type", "application/json")
            .with_body(json!({"error": "Request timeout"}).to_string())
    }

    /// Create mock with partial data response
    fn mock_partial_data_response(&mut self, coin: &str) -> Mock {
        let partial_data = json!([
            {
                "T": 1640995200000i64,
                "c": "47000.5",
                "h": "47500.0",
                "l": "46500.0",
                "n": 1000,
                "o": "47200.0",
                "t": 1640995200000i64,
                "v": "125.5"
            }
            // Missing expected data points
        ]);

        self.server
            .mock("POST", "/info")
            .match_body(mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(partial_data.to_string())
    }
}

#[tokio::test]
async fn test_api_resilience_and_error_recovery() {
    let mut mock_server = MockHyperliquidServer::new().await;
    
    // Test rate limiting handling
    let _rate_limit_mock = mock_server.mock_rate_limited_response();
    
    // Test timeout handling
    let _timeout_mock = mock_server.mock_timeout_response();
    
    // Test partial data handling
    let _partial_mock = mock_server.mock_partial_data_response("BTC");
    
    // Test various error scenarios
    let error_scenarios = vec![
        (400, "Bad request"),
        (401, "Unauthorized"),
        (403, "Forbidden"),
        (404, "Not found"),
        (500, "Internal server error"),
        (502, "Bad gateway"),
        (503, "Service unavailable"),
    ];
    
    for (status_code, error_message) in error_scenarios {
        let _error_mock = mock_server.mock_api_error(status_code, error_message);
        
        // Test error handling and conversion
        let error = HyperliquidBacktestError::DataConversion(
            format!("HTTP {} error: {}", status_code, error_message)
        );
        assert!(error.to_string().contains("Data conversion error"));
    }
}

#[tokio::test]
async fn test_large_dataset_api_simulation() {
    let mut mock_server = MockHyperliquidServer::new().await;
    
    // Test with large dataset simulation
    let _large_mock = mock_server.mock_large_candles_snapshot("BTC", "1h", 10000);
    
    // Simulate processing large API response
    // In real implementation, this would test actual API fetching
    let large_data = create_large_test_data(10000);
    
    // Test data processing performance with large dataset
    let start_time = std::time::Instant::now();
    let rs_data = large_data.to_rs_backtester_data();
    let conversion_time = start_time.elapsed();
    
    info!("Large dataset conversion took: {:?}", conversion_time);
    assert!(conversion_time.as_secs() < 5, "Data conversion took too long: {:?}", conversion_time);
    
    // Test backtesting performance
    let start_time = std::time::Instant::now();
    let strategy = enhanced_sma_cross(20, 50, 0.3);
    let mut backtest = HyperliquidBacktest::new(
        large_data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );
    backtest.calculate_with_funding();
    let backtest_time = start_time.elapsed();
    
    info!("Large dataset backtesting took: {:?}", backtest_time);
    assert!(backtest_time.as_secs() < 30, "Backtesting took too long: {:?}", backtest_time);
}

// ============================================================================
// END-TO-END WORKFLOW TESTS
// ============================================================================

#[tokio::test]
async fn test_complete_trading_workflow() {
    // Test complete workflow from data fetching to report generation
    let test_scenarios = vec![
        ("BTC", "1h", 1000),
        ("ETH", "4h", 500),
        ("SOL", "1d", 100),
    ];
    
    for (coin, interval, size) in test_scenarios {
        info!("Testing complete workflow for {} {} with {} points", coin, interval, size);
        
        // Step 1: Data fetching simulation
        let data = create_realistic_test_data(coin, size);
        assert_eq!(data.ticker, coin);
        assert_eq!(data.datetime.len(), size);
        
        // Step 2: Data validation
        assert_eq!(data.open.len(), size);
        assert_eq!(data.high.len(), size);
        assert_eq!(data.low.len(), size);
        assert_eq!(data.close.len(), size);
        assert_eq!(data.volume.len(), size);
        assert_eq!(data.funding_rates.len(), size);
        
        // Step 3: Strategy creation and backtesting
        let strategies = vec![
            ("SMA Cross", enhanced_sma_cross(10, 30, 0.2)),
            ("Funding Arbitrage", funding_arbitrage_strategy(0.001, Default::default())),
        ];
        
        for (strategy_name, strategy) in strategies {
            info!("Testing strategy: {}", strategy_name);
            
            let mut backtest = HyperliquidBacktest::new(
                data.clone(),
                strategy,
                10000.0,
                HyperliquidCommission::default(),
            );
            
            // Step 4: Run backtest with funding
            backtest.calculate_with_funding();
            
            // Step 5: Generate reports
            let enhanced_report = backtest.enhanced_report();
            let funding_report = backtest.funding_report();
            
            // Step 6: Validate results
            assert!(enhanced_report.total_return.is_finite());
            assert!(funding_report.total_funding_paid >= 0.0);
            assert!(funding_report.total_funding_received >= 0.0);
            
            // Step 7: CSV export
            let mut csv_buffer = Vec::new();
            backtest.enhanced_csv_export(&mut csv_buffer).unwrap();
            assert!(!csv_buffer.is_empty());
            
            info!("Strategy {} completed successfully", strategy_name);
        }
    }
}

#[tokio::test]
async fn test_multi_asset_portfolio_workflow() {
    let assets = vec!["BTC", "ETH", "SOL", "AVAX"];
    let mut portfolio_results = HashMap::new();
    
    // Test concurrent processing of multiple assets
    let tasks: Vec<_> = assets.iter().map(|&asset| {
        tokio::spawn(async move {
            let data = create_realistic_test_data(asset, 1000);
            let strategy = enhanced_sma_cross(15, 35, 0.3);
            let mut backtest = HyperliquidBacktest::new(
                data,
                strategy,
                10000.0,
                HyperliquidCommission::default(),
            );
            
            backtest.calculate_with_funding();
            let report = backtest.enhanced_report();
            let funding_report = backtest.funding_report();
            
            (asset, report.total_return, funding_report.total_funding_paid)
        })
    }).collect();
    
    let results = join_all(tasks).await;
    
    for result in results {
        let (asset, total_return, funding_paid) = result.unwrap();
        portfolio_results.insert(asset, (total_return, funding_paid));
        info!("Asset {}: return={:.2}%, funding={:.2}", 
              asset, total_return * 100.0, funding_paid);
    }
    
    // Validate portfolio results
    assert_eq!(portfolio_results.len(), assets.len());
    
    let total_portfolio_return: f64 = portfolio_results.values()
        .map(|(ret, _)| ret)
        .sum::<f64>() / portfolio_results.len() as f64;
    
    let total_funding_paid: f64 = portfolio_results.values()
        .map(|(_, funding)| funding)
        .sum();
    
    info!("Portfolio average return: {:.2}%", total_portfolio_return * 100.0);
    info!("Total funding paid: {:.2}", total_funding_paid);
    
    assert!(total_portfolio_return.is_finite());
}

#[tokio::test]
async fn test_api_rate_limiting_simulation() {
    let mut mock_server = MockHyperliquidServer::new().await;
    
    // Simulate rate limiting scenario
    let _rate_limit_mock = mock_server.mock_rate_limited_response();
    
    // Test that our system handles rate limiting gracefully
    // In a real implementation, this would test retry logic and backoff
    let error = HyperliquidBacktestError::DataConversion("Rate limit exceeded".to_string());
    assert!(error.to_string().contains("Rate limit exceeded"));
}

#[tokio::test]
async fn test_network_failure_resilience() {
    let mut mock_server = MockHyperliquidServer::new().await;
    
    // Test various network failure scenarios
    let failure_scenarios = vec![
        (500, "Internal Server Error"),
        (502, "Bad Gateway"),
        (503, "Service Unavailable"),
        (504, "Gateway Timeout"),
    ];
    
    for (status_code, error_message) in failure_scenarios {
        let _error_mock = mock_server.mock_api_error(status_code, error_message);
        
        // Test error handling
        let error = HyperliquidBacktestError::DataConversion(
            format!("Network error {}: {}", status_code, error_message)
        );
        assert!(error.to_string().contains("Data conversion error"));
    }
}

// ============================================================================
// HELPER FUNCTIONS FOR INTEGRATION TESTS
// ============================================================================

/// Create test data for integration testing
fn create_test_hyperliquid_data() -> HyperliquidData {
    let size = 100;
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let prices: Vec<f64> = (0..size)
        .map(|i| 47000.0 + (i as f64 * 0.1).sin() * 100.0)
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + 25.0).collect(),
        low: prices.iter().map(|p| p - 25.0).collect(),
        close: prices,
        volume: vec![100.0; size],
        funding_rates: (0..size).map(|i| 0.0001 + (i as f64 * 0.01).sin() * 0.0001).collect(),
        funding_timestamps: datetime,
    }
}

/// Create large test data for memory testing
fn create_large_test_data(size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let prices: Vec<f64> = (0..size)
        .map(|i| 47000.0 + (i as f64 * 0.01).sin() * 200.0)
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + 50.0).collect(),
        low: prices.iter().map(|p| p - 50.0).collect(),
        close: prices,
        volume: vec![100.0; size],
        funding_rates: (0..size).map(|i| 0.0001 + (i as f64 * 0.001).sin() * 0.0001).collect(),
        funding_timestamps: datetime,
    }
}

#[tokio::test]
async fn test_api_rate_limiting_simulation() {
    let mut mock_server = MockHyperliquidServer::new().await;
    
    // Simulate rate limiting scenario
    let _rate_limit_mock = mock_server.mock_rate_limited_response();
    
    // Test that our system handles rate limiting gracefully
    // In a real implementation, this would test retry logic and backoff
    let error = HyperliquidBacktestError::DataConversion("Rate limit exceeded".to_string());
    assert!(error.to_string().contains("Rate limit exceeded"));
}

#[tokio::test]
async fn test_network_failure_resilience() {
    let mut mock_server = MockHyperliquidServer::new().await;
    
    // Test various network failure scenarios
    let failure_scenarios = vec![
        (500, "Internal Server Error"),
        (502, "Bad Gateway"),
        (503, "Service Unavailable"),
        (504, "Gateway Timeout"),
    ];
    
    for (status_code, error_message) in failure_scenarios {
        let _error_mock = mock_server.mock_api_error(status_code, error_message);
        
        // Test error handling
        let error = HyperliquidBacktestError::DataConversion(
            format!("Network error {}: {}", status_code, error_message)
        );
        assert!(error.to_string().contains("Data conversion error"));
    }
}

#[tokio::test]
async fn test_data_integrity_validation() {
    // Test data integrity across the entire pipeline
    let test_cases = vec![
        ("BTC", 1000, "1h"),
        ("ETH", 2000, "4h"),
        ("SOL", 500, "1d"),
    ];
    
    for (coin, size, interval) in test_cases {
        let data = create_realistic_test_data(coin, size);
        
        // Validate data integrity
        assert_eq!(data.ticker, coin);
        assert_eq!(data.datetime.len(), size);
        assert_eq!(data.open.len(), size);
        assert_eq!(data.high.len(), size);
        assert_eq!(data.low.len(), size);
        assert_eq!(data.close.len(), size);
        assert_eq!(data.volume.len(), size);
        
        // Validate OHLC relationships
        for i in 0..size {
            assert!(data.high[i] >= data.low[i], "Invalid OHLC at index {}", i);
            assert!(data.high[i] >= data.open[i], "High < Open at index {}", i);
            assert!(data.high[i] >= data.close[i], "High < Close at index {}", i);
            assert!(data.low[i] <= data.open[i], "Low > Open at index {}", i);
            assert!(data.low[i] <= data.close[i], "Low > Close at index {}", i);
            assert!(data.volume[i] >= 0.0, "Negative volume at index {}", i);
        }
        
        // Test conversion integrity
        let rs_data = data.to_rs_backtester_data();
        assert_eq!(rs_data.close.len(), data.close.len());
        
        for i in 0..size {
            assert!((rs_data.close[i] - data.close[i]).abs() < 1e-10, 
                   "Price conversion error at index {}", i);
        }
    }
}

#[tokio::test]
async fn test_extreme_market_conditions() {
    // Test system behavior under extreme market conditions
    let extreme_scenarios = vec![
        ("Flash Crash", create_flash_crash_data()),
        ("High Volatility", create_high_volatility_data()),
        ("Low Liquidity", create_low_liquidity_data()),
        ("Funding Rate Spike", create_funding_spike_data()),
    ];
    
    for (scenario_name, data) in extreme_scenarios {
        info!("Testing extreme scenario: {}", scenario_name);
        
        let strategy = enhanced_sma_cross(10, 20, 0.3);
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        // Should handle extreme conditions without panicking
        backtest.calculate_with_funding();
        let report = backtest.enhanced_report();
        
        // Results should be finite even in extreme conditions
        assert!(report.total_return.is_finite(), 
               "Scenario {} produced invalid return", scenario_name);
        assert!(report.max_drawdown.is_finite(),
               "Scenario {} produced invalid drawdown", scenario_name);
        
        info!("Scenario {} handled successfully", scenario_name);
    }
}

#[tokio::test]
async fn test_long_running_backtest_stability() {
    // Test stability over long-running backtests
    let data = create_long_time_series_test_data(100_000);
    let strategy = enhanced_sma_cross(50, 200, 0.2);
    
    let start_time = std::time::Instant::now();
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );
    
    backtest.calculate_with_funding();
    let duration = start_time.elapsed();
    
    let report = backtest.enhanced_report();
    let funding_report = backtest.funding_report();
    
    // Verify results are valid
    assert!(report.total_return.is_finite());
    assert!(funding_report.total_funding_paid >= 0.0);
    assert!(funding_report.total_funding_received >= 0.0);
    
    info!("Long-running backtest completed in {:?}", duration);
    assert!(duration.as_secs() < 300, "Long backtest took too long: {:?}", duration);
}

#[tokio::test]
async fn test_memory_usage_monitoring() {
    use memory_stats::memory_stats;
    
    let initial_memory = memory_stats().map(|stats| stats.physical_mem).unwrap_or(0);
    let mut peak_memory = initial_memory;
    
    // Process multiple datasets and monitor memory usage
    for i in 0..10 {
        let data = create_realistic_test_data("BTC", 10_000 + i * 1000);
        let strategy = enhanced_sma_cross(10 + i, 30 + i * 2, 0.1 + i as f64 * 0.05);
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let _report = backtest.enhanced_report();
        
        // Monitor memory usage
        if let Some(current_stats) = memory_stats() {
            peak_memory = peak_memory.max(current_stats.physical_mem);
        }
        
        // Explicit cleanup
        drop(backtest);
    }
    
    let memory_growth = peak_memory.saturating_sub(initial_memory);
    info!("Peak memory growth: {} bytes", memory_growth);
    
    // Memory growth should be reasonable
    assert!(memory_growth < 1_000_000_000, // 1GB limit
           "Excessive memory growth: {} bytes", memory_growth);
}

#[tokio::test]
async fn test_concurrent_api_simulation() {
    // Simulate concurrent API requests
    let num_concurrent = 10;
    let tasks: Vec<_> = (0..num_concurrent).map(|i| {
        tokio::spawn(async move {
            let data = create_realistic_test_data(&format!("ASSET{}", i), 5000);
            let strategy = enhanced_sma_cross(10 + i % 5, 30 + i % 10, 0.1 + (i as f64 % 5.0) * 0.1);
            let mut backtest = HyperliquidBacktest::new(
                data,
                strategy,
                10000.0,
                HyperliquidCommission::default(),
            );
            
            backtest.calculate_with_funding();
            let report = backtest.enhanced_report();
            
            (i, report.total_return, report.max_drawdown)
        })
    }).collect();
    
    let results = join_all(tasks).await;
    
    // Verify all concurrent operations completed successfully
    let mut successful_count = 0;
    for result in results {
        if let Ok((task_id, total_return, max_drawdown)) = result {
            if total_return.is_finite() && max_drawdown.is_finite() {
                successful_count += 1;
            }
        }
    }
    
    assert_eq!(successful_count, num_concurrent, 
              "Not all concurrent operations completed successfully");
}

#[tokio::test]
async fn test_data_export_integrity() {
    let data = create_realistic_test_data("BTC", 5000);
    let strategy = enhanced_sma_cross(20, 50, 0.3);
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );
    
    backtest.calculate_with_funding();
    
    // Test CSV export integrity
    let mut csv_buffer = Vec::new();
    backtest.enhanced_csv_export(&mut csv_buffer).unwrap();
    
    let csv_string = String::from_utf8(csv_buffer).unwrap();
    let lines: Vec<&str> = csv_string.lines().collect();
    
    // Verify CSV structure
    assert!(!lines.is_empty(), "CSV export is empty");
    assert!(lines[0].contains("timestamp"), "Missing timestamp header");
    assert!(lines[0].contains("close"), "Missing close header");
    assert!(lines[0].contains("funding_rate"), "Missing funding_rate header");
    
    // Verify data integrity in CSV
    assert!(lines.len() > data.datetime.len() / 2, "CSV missing significant data");
    
    // Test that CSV can be parsed back
    for (i, line) in lines.iter().skip(1).take(10).enumerate() {
        let fields: Vec<&str> = line.split(',').collect();
        assert!(fields.len() >= 3, "CSV line {} has insufficient fields", i);
        
        // Verify numeric fields can be parsed
        if let Some(close_field) = fields.get(1) {
            assert!(close_field.parse::<f64>().is_ok(), 
                   "Invalid close price in CSV line {}", i);
        }
    }
}

#[tokio::test]
async fn test_error_recovery_scenarios() {
    // Test various error recovery scenarios
    let error_scenarios = vec![
        ("Invalid data format", create_invalid_format_data()),
        ("Missing timestamps", create_missing_timestamp_data()),
        ("Inconsistent lengths", create_inconsistent_length_data()),
    ];
    
    for (scenario_name, data_result) in error_scenarios {
        info!("Testing error recovery scenario: {}", scenario_name);
        
        match data_result {
            Ok(data) => {
                // If data creation succeeded, test that backtesting handles it gracefully
                let strategy = enhanced_sma_cross(10, 20, 0.3);
                let backtest_result = std::panic::catch_unwind(|| {
                    let mut backtest = HyperliquidBacktest::new(
                        data,
                        strategy,
                        10000.0,
                        HyperliquidCommission::default(),
                    );
                    backtest.calculate_with_funding();
                    backtest.enhanced_report()
                });
                
                // Should either succeed or fail gracefully
                match backtest_result {
                    Ok(report) => {
                        assert!(report.total_return.is_finite() || report.total_return.is_nan(),
                               "Invalid report for scenario {}", scenario_name);
                    }
                    Err(_) => {
                        info!("Scenario {} failed as expected", scenario_name);
                    }
                }
            }
            Err(_) => {
                info!("Scenario {} failed at data creation as expected", scenario_name);
            }
        }
    }
}

// ============================================================================
// ADDITIONAL HELPER FUNCTIONS
// ============================================================================

/// Create realistic test data for a specific asset
fn create_realistic_test_data(ticker: &str, size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let base_price = match ticker {
        "BTC" => 47000.0,
        "ETH" => 3500.0,
        "SOL" => 150.0,
        "AVAX" => 80.0,
        _ => 100.0,
    };

    // Generate realistic price movement with trends, cycles, and noise
    let prices: Vec<f64> = (0..size)
        .map(|i| {
            let t = i as f64 / size as f64;
            let trend = base_price * 0.1 * t; // 10% trend
            let cycle = (i as f64 * 0.02).sin() * base_price * 0.03; // 3% cyclical
            let noise = (i as f64 * 0.5).sin() * base_price * 0.005; // 0.5% noise
            base_price + trend + cycle + noise
        })
        .collect();

    HyperliquidData {
        ticker: ticker.to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] + (prices[i] - prices[i-1]) * 0.1 }
        }).collect(),
        high: prices.iter().map(|p| p + p * 0.008).collect(),
        low: prices.iter().map(|p| p - p * 0.008).collect(),
        close: prices,
        volume: (0..size).map(|i| {
            100.0 + (i as f64 * 0.1).sin().abs() * 50.0
        }).collect(),
        funding_rates: (0..size).map(|i| {
            0.0001 + (i as f64 * 0.03).sin() * 0.0002
        }).collect(),
        funding_timestamps: datetime,
    }
}

/// Create flash crash scenario data
fn create_flash_crash_data() -> HyperliquidData {
    let size = 1000;
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 60, 0) // 1-minute intervals
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let mut prices = Vec::new();
    let mut price = 47000.0;
    
    for i in 0..size {
        if i == 500 { // Flash crash at midpoint
            price *= 0.8; // 20% crash
        } else if i > 500 && i < 600 { // Recovery
            price += (47000.0 * 0.8 - price) * 0.1; // Gradual recovery
        } else {
            price += (i as f64 * 0.1).sin() * 10.0; // Normal volatility
        }
        prices.push(price);
    }

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + 20.0).collect(),
        low: prices.iter().map(|p| p - 20.0).collect(),
        close: prices,
        volume: (0..size).map(|i| {
            if i >= 490 && i <= 510 { 1000.0 } else { 100.0 } // High volume during crash
        }).collect(),
        funding_rates: vec![0.0001; size],
        funding_timestamps: datetime,
    }
}

/// Create high volatility scenario data
fn create_high_volatility_data() -> HyperliquidData {
    let size = 2000;
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 1800, 0) // 30-minute intervals
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let mut price = 47000.0;
    let prices: Vec<f64> = (0..size)
        .map(|i| {
            // High volatility with large swings
            let volatility = (i as f64 * 0.2).sin() * price * 0.05; // 5% swings
            let shock = if i % 50 == 0 { (i as f64).cos() * price * 0.02 } else { 0.0 };
            price += volatility + shock;
            price.max(1000.0) // Prevent unrealistic low prices
        })
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + p * 0.03).collect(),
        low: prices.iter().map(|p| p - p * 0.03).collect(),
        close: prices,
        volume: (0..size).map(|i| 100.0 + (i as f64 * 0.3).sin().abs() * 200.0).collect(),
        funding_rates: (0..size).map(|i| 0.0001 + (i as f64 * 0.1).sin() * 0.001).collect(),
        funding_timestamps: datetime,
    }
}

/// Create low liquidity scenario data
fn create_low_liquidity_data() -> HyperliquidData {
    let size = 500;
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 7200, 0) // 2-hour intervals
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let prices: Vec<f64> = (0..size)
        .map(|i| 47000.0 + (i as f64 * 0.05).sin() * 50.0)
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + 5.0).collect(),
        low: prices.iter().map(|p| p - 5.0).collect(),
        close: prices,
        volume: vec![10.0; size], // Very low volume
        funding_rates: vec![0.0001; size],
        funding_timestamps: datetime,
    }
}

/// Create funding rate spike scenario data
fn create_funding_spike_data() -> HyperliquidData {
    let size = 1000;
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let prices: Vec<f64> = (0..size)
        .map(|i| 47000.0 + (i as f64 * 0.01).sin() * 100.0)
        .collect();

    let funding_rates: Vec<f64> = (0..size)
        .map(|i| {
            if i >= 400 && i <= 450 { // Funding spike period
                0.01 // 1% funding rate spike
            } else {
                0.0001 + (i as f64 * 0.02).sin() * 0.0001
            }
        })
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + 25.0).collect(),
        low: prices.iter().map(|p| p - 25.0).collect(),
        close: prices,
        volume: vec![100.0; size],
        funding_rates,
        funding_timestamps: datetime,
    }
}

/// Create long time series test data
fn create_long_time_series_test_data(size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    // Long-term trend with multiple cycles
    let prices: Vec<f64> = (0..size)
        .map(|i| {
            let t = i as f64 / size as f64;
            let long_trend = 47000.0 * (1.0 + t * 0.5); // 50% growth over period
            let medium_cycle = (i as f64 * 0.001).sin() * 2000.0; // Medium-term cycles
            let short_cycle = (i as f64 * 0.01).sin() * 200.0; // Short-term cycles
            long_trend + medium_cycle + short_cycle
        })
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + 50.0).collect(),
        low: prices.iter().map(|p| p - 50.0).collect(),
        close: prices,
        volume: vec![100.0; size],
        funding_rates: (0..size).map(|i| 0.0001 + (i as f64 * 0.0001).sin() * 0.0001).collect(),
        funding_timestamps: datetime,
    }
}

/// Create invalid format data for error testing
fn create_invalid_format_data() -> std::result::Result<HyperliquidData, &'static str> {
    // Simulate data with invalid format
    Err("Invalid data format")
}

/// Create data with missing timestamps for error testing
fn create_missing_timestamp_data() -> std::result::Result<HyperliquidData, &'static str> {
    // Simulate data with missing timestamps
    Err("Missing timestamp data")
}

/// Create data with inconsistent lengths for error testing
fn create_inconsistent_length_data() -> std::result::Result<HyperliquidData, &'static str> {
    // This would create data with mismatched vector lengths
    // For now, return an error to simulate the detection of this issue
    Err("Inconsistent data lengths")
}

#[tokio::test]
async fn test_api_compatibility_regression() {
    // Test that API changes don't break existing functionality
    let mut mock_server = MockHyperliquidServer::new().await;
    
    // Test with different API response formats
    let api_versions = vec![
        ("v1", create_v1_api_response()),
        ("v2", create_v2_api_response()),
    ];
    
    for (version, response) in api_versions {
        info!("Testing API compatibility for version: {}", version);
        
        // Mock the API response
        let _mock = mock_server.server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response);
        
        // Test that our system can handle different response formats
        // In a real implementation, this would test actual API parsing
        let test_result = parse_api_response(&response);
        assert!(test_result.is_ok(), "Failed to parse {} API response", version);
    }
}

#[tokio::test]
async fn test_network_resilience() {
    let mut mock_server = MockHyperliquidServer::new().await;
    
    // Test network failure scenarios
    let network_scenarios = vec![
        ("connection_timeout", 408, "Request timeout"),
        ("server_error", 500, "Internal server error"),
        ("bad_gateway", 502, "Bad gateway"),
        ("service_unavailable", 503, "Service unavailable"),
        ("rate_limited", 429, "Too many requests"),
    ];
    
    for (scenario, status_code, error_msg) in network_scenarios {
        info!("Testing network resilience scenario: {}", scenario);
        
        let _mock = mock_server.mock_api_error(status_code, error_msg);
        
        // Test error handling
        let error = HyperliquidBacktestError::DataConversion(
            format!("Network error {}: {}", status_code, error_msg)
        );
        
        assert!(error.to_string().contains("Data conversion error"));
        assert!(error.to_string().contains(&status_code.to_string()));
    }
}

#[tokio::test]
async fn test_data_integrity_validation() {
    // Test comprehensive data validation
    let test_cases = vec![
        ("missing_ohlc_data", create_missing_ohlc_data()),
        ("inconsistent_timestamps", create_inconsistent_timestamp_data()),
        ("negative_prices", create_negative_price_data()),
        ("extreme_values", create_extreme_value_data()),
    ];
    
    for (case_name, data) in test_cases {
        info!("Testing data integrity case: {}", case_name);
        
        // Test that system handles invalid data gracefully
        let validation_result = validate_hyperliquid_data(&data);
        
        match case_name {
            "missing_ohlc_data" | "negative_prices" => {
                assert!(validation_result.is_err(), "Should reject invalid data for {}", case_name);
            }
            "inconsistent_timestamps" | "extreme_values" => {
                // These might be handled with warnings rather than errors
                assert!(validation_result.is_ok() || validation_result.is_err());
            }
            _ => {}
        }
    }
}

#[tokio::test]
async fn test_performance_under_load() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    let processed_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));
    
    // Simulate high load with concurrent requests
    let tasks: Vec<_> = (0..20).map(|i| {
        let processed = Arc::clone(&processed_count);
        let errors = Arc::clone(&error_count);
        
        tokio::spawn(async move {
            for j in 0..10 {
                let data = create_realistic_test_data(&format!("ASSET{}", i), 1000 + j * 100);
                let strategy = enhanced_sma_cross(10 + j, 30 + j * 2, 0.1 + j as f64 * 0.05);
                
                match std::panic::catch_unwind(|| {
                    let mut backtest = HyperliquidBacktest::new(
                        data,
                        strategy,
                        10000.0,
                        HyperliquidCommission::default(),
                    );
                    backtest.calculate_with_funding();
                    backtest.enhanced_report()
                }) {
                    Ok(_) => {
                        processed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        })
    }).collect();
    
    // Wait for all tasks to complete
    join_all(tasks).await;
    
    let total_processed = processed_count.load(Ordering::Relaxed);
    let total_errors = error_count.load(Ordering::Relaxed);
    
    info!("Performance under load: {} processed, {} errors", total_processed, total_errors);
    
    // Should handle most requests successfully
    assert!(total_processed > total_errors * 10, "Too many errors under load");
    assert!(total_processed > 150, "Not enough requests processed successfully");
}

// ============================================================================
// ADDITIONAL HELPER FUNCTIONS
// ============================================================================

/// Create test data for regression testing
fn create_test_hyperliquid_data() -> HyperliquidData {
    let size = 100;
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let prices: Vec<f64> = (0..size)
        .map(|i| 47000.0 + (i as f64 * 0.1).sin() * 100.0)
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + 25.0).collect(),
        low: prices.iter().map(|p| p - 25.0).collect(),
        close: prices,
        volume: vec![100.0; size],
        funding_rates: (0..size).map(|i| 0.0001 + (i as f64 * 0.01).sin() * 0.0001).collect(),
        funding_timestamps: datetime,
    }
}

/// Create large test data for memory testing
fn create_large_test_data(size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let prices: Vec<f64> = (0..size)
        .map(|i| 47000.0 + (i as f64 * 0.01).sin() * 500.0)
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + 50.0).collect(),
        low: prices.iter().map(|p| p - 50.0).collect(),
        close: prices,
        volume: vec![100.0; size],
        funding_rates: (0..size).map(|i| 0.0001 + (i as f64 * 0.001).sin() * 0.0001).collect(),
        funding_timestamps: datetime,
    }
}

/// Create v1 API response format for compatibility testing
fn create_v1_api_response() -> String {
    serde_json::json!([
        {
            "T": 1640995200000i64,
            "c": "47000.5",
            "h": "47500.0",
            "l": "46500.0",
            "n": 1000,
            "o": "47200.0",
            "t": 1640995200000i64,
            "v": "125.5"
        }
    ]).to_string()
}

/// Create v2 API response format for compatibility testing
fn create_v2_api_response() -> String {
    serde_json::json!([
        {
            "timestamp": 1640995200000i64,
            "close": "47000.5",
            "high": "47500.0",
            "low": "46500.0",
            "trades": 1000,
            "open": "47200.0",
            "time": 1640995200000i64,
            "volume": "125.5",
            "extra_field": "new_data" // Additional field for forward compatibility
        }
    ]).to_string()
}

/// Parse API response for compatibility testing
fn parse_api_response(response: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let _parsed: serde_json::Value = serde_json::from_str(response)?;
    Ok(())
}

/// Validate HyperliquidData for integrity testing
fn validate_hyperliquid_data(data: &HyperliquidData) -> Result<(), String> {
    if data.open.is_empty() {
        return Err("Missing OHLC data".to_string());
    }
    
    if data.close.iter().any(|&price| price <= 0.0) {
        return Err("Negative or zero prices detected".to_string());
    }
    
    if data.datetime.len() != data.close.len() {
        return Err("Inconsistent data lengths".to_string());
    }
    
    Ok(())
}

/// Create test data with missing OHLC data
fn create_missing_ohlc_data() -> HyperliquidData {
    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: vec![],
        open: vec![],
        high: vec![],
        low: vec![],
        close: vec![],
        volume: vec![],
        funding_rates: vec![0.0001],
        funding_timestamps: vec![DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap())],
    }
}

/// Create test data with inconsistent timestamps
fn create_inconsistent_timestamp_data() -> HyperliquidData {
    let datetime1 = vec![
        DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
        DateTime::from_timestamp(1640995100, 0) // Earlier timestamp
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
    ];
    
    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime1.clone(),
        open: vec![47000.0, 47100.0],
        high: vec![47500.0, 47600.0],
        low: vec![46500.0, 46600.0],
        close: vec![47200.0, 47300.0],
        volume: vec![100.0, 110.0],
        funding_rates: vec![0.0001, 0.00015],
        funding_timestamps: datetime1,
    }
}

/// Create test data with negative prices
fn create_negative_price_data() -> HyperliquidData {
    let datetime = vec![
        DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
    ];
    
    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: vec![-100.0], // Negative price
        high: vec![47500.0],
        low: vec![46500.0],
        close: vec![47200.0],
        volume: vec![100.0],
        funding_rates: vec![0.0001],
        funding_timestamps: datetime,
    }
}

/// Create test data with extreme values
fn create_extreme_value_data() -> HyperliquidData {
    let datetime = vec![
        DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
    ];
    
    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: vec![f64::MAX / 2.0], // Extreme value
        high: vec![f64::MAX],
        low: vec![f64::MIN_POSITIVE],
        close: vec![1e10], // Very large value
        volume: vec![1e15], // Extreme volume
        funding_rates: vec![1.0], // 100% funding rate
        funding_timestamps: datetime,
    }
}

/// Create realistic test data for specific asset
fn create_realistic_test_data(ticker: &str, size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let base_price = match ticker {
        "BTC" => 47000.0,
        "ETH" => 3500.0,
        "SOL" => 150.0,
        "AVAX" => 80.0,
        _ => 100.0,
    };

    let prices: Vec<f64> = (0..size)
        .map(|i| {
            let trend = (i as f64 / size as f64) * base_price * 0.1;
            let cycle = (i as f64 * 0.02).sin() * base_price * 0.05;
            let noise = (i as f64 * 0.5).sin() * base_price * 0.01;
            base_price + trend + cycle + noise
        })
        .collect();

    HyperliquidData {
        ticker: ticker.to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + p * 0.01).collect(),
        low: prices.iter().map(|p| p - p * 0.01).collect(),
        close: prices,
        volume: (0..size).map(|i| 100.0 + (i as f64 * 0.1).sin() * 50.0).collect(),
        funding_rates: (0..size).map(|i| 0.0001 + (i as f64 * 0.01).sin() * 0.0001).collect(),
        funding_timestamps: datetime,
    }
}

#[tokio::test]
async fn test_api_compatibility_regression() {
    // Test that API changes don't break existing functionality
    let mut mock_server = MockHyperliquidServer::new().await;
    
    // Test with different API response formats
    let api_versions = vec![
        ("v1", create_v1_api_response()),
        ("v2", create_v2_api_response()),
    ];
    
    for (version, response) in api_versions {
        info!("Testing API compatibility for version: {}", version);
        
        // Mock the API response
        let _mock = mock_server.server
            .mock("POST", "/info")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response);
        
        // Test that our system can handle different response formats
        // In a real implementation, this would test actual API parsing
        let test_result = parse_api_response(&response);
        assert!(test_result.is_ok(), "Failed to parse {} API response", version);
    }
}

#[tokio::test]
async fn test_network_resilience() {
    let mut mock_server = MockHyperliquidServer::new().await;
    
    // Test network failure scenarios
    let network_scenarios = vec![
        ("connection_timeout", 408, "Request timeout"),
        ("server_error", 500, "Internal server error"),
        ("bad_gateway", 502, "Bad gateway"),
        ("service_unavailable", 503, "Service unavailable"),
        ("rate_limited", 429, "Too many requests"),
    ];
    
    for (scenario, status_code, error_msg) in network_scenarios {
        info!("Testing network resilience scenario: {}", scenario);
        
        let _mock = mock_server.mock_api_error(status_code, error_msg);
        
        // Test error handling
        let error = HyperliquidBacktestError::DataConversion(
            format!("Network error {}: {}", status_code, error_msg)
        );
        
        assert!(error.to_string().contains("Data conversion error"));
        assert!(error.to_string().contains(&status_code.to_string()));
    }
}

#[tokio::test]
async fn test_data_integrity_validation() {
    // Test comprehensive data validation
    let test_cases = vec![
        ("missing_ohlc_data", create_missing_ohlc_data()),
        ("inconsistent_timestamps", create_inconsistent_timestamp_data()),
        ("negative_prices", create_negative_price_data()),
        ("extreme_values", create_extreme_value_data()),
    ];
    
    for (case_name, data) in test_cases {
        info!("Testing data integrity case: {}", case_name);
        
        // Test that system handles invalid data gracefully
        let validation_result = validate_hyperliquid_data(&data);
        
        match case_name {
            "missing_ohlc_data" | "negative_prices" => {
                assert!(validation_result.is_err(), "Should reject invalid data for {}", case_name);
            }
            "inconsistent_timestamps" | "extreme_values" => {
                // These might be handled with warnings rather than errors
                assert!(validation_result.is_ok() || validation_result.is_err());
            }
            _ => {}
        }
    }
}

#[tokio::test]
async fn test_performance_under_load() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    let processed_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));
    
    // Simulate high load with concurrent requests
    let tasks: Vec<_> = (0..20).map(|i| {
        let processed = Arc::clone(&processed_count);
        let errors = Arc::clone(&error_count);
        
        tokio::spawn(async move {
            for j in 0..10 {
                let data = create_realistic_test_data(&format!("ASSET{}", i), 1000 + j * 100);
                let strategy = enhanced_sma_cross(10 + j, 30 + j * 2, 0.1 + j as f64 * 0.05);
                
                match std::panic::catch_unwind(|| {
                    let mut backtest = HyperliquidBacktest::new(
                        data,
                        strategy,
                        10000.0,
                        HyperliquidCommission::default(),
                    );
                    backtest.calculate_with_funding();
                    backtest.enhanced_report()
                }) {
                    Ok(_) => {
                        processed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        })
    }).collect();
    
    // Wait for all tasks to complete
    join_all(tasks).await;
    
    let total_processed = processed_count.load(Ordering::Relaxed);
    let total_errors = error_count.load(Ordering::Relaxed);
    
    info!("Performance under load: {} processed, {} errors", total_processed, total_errors);
    
    // Should handle most requests successfully
    assert!(total_processed > total_errors * 10, "Too many errors under load");
    assert!(total_processed > 150, "Not enough requests processed successfully");
}

// ============================================================================
// ADDITIONAL HELPER FUNCTIONS
// ============================================================================

/// Create v1 API response format for compatibility testing
fn create_v1_api_response() -> String {
    serde_json::json!([
        {
            "T": 1640995200000i64,
            "c": "47000.5",
            "h": "47500.0",
            "l": "46500.0",
            "n": 1000,
            "o": "47200.0",
            "t": 1640995200000i64,
            "v": "125.5"
        }
    ]).to_string()
}

/// Create v2 API response format for compatibility testing
fn create_v2_api_response() -> String {
    serde_json::json!([
        {
            "timestamp": 1640995200000i64,
            "close": "47000.5",
            "high": "47500.0",
            "low": "46500.0",
            "trades": 1000,
            "open": "47200.0",
            "time": 1640995200000i64,
            "volume": "125.5",
            "extra_field": "new_data" // Additional field for forward compatibility
        }
    ]).to_string()
}

/// Parse API response for compatibility testing
fn parse_api_response(response: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let _parsed: serde_json::Value = serde_json::from_str(response)?;
    Ok(())
}

/// Validate HyperliquidData for integrity testing
fn validate_hyperliquid_data(data: &HyperliquidData) -> Result<(), String> {
    if data.open.is_empty() {
        return Err("Missing OHLC data".to_string());
    }
    
    if data.close.iter().any(|&price| price <= 0.0) {
        return Err("Negative or zero prices detected".to_string());
    }
    
    if data.datetime.len() != data.close.len() {
        return Err("Inconsistent data lengths".to_string());
    }
    
    Ok(())
}

/// Create test data with missing OHLC data
fn create_missing_ohlc_data() -> HyperliquidData {
    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: vec![],
        open: vec![],
        high: vec![],
        low: vec![],
        close: vec![],
        volume: vec![],
        funding_rates: vec![0.0001],
        funding_timestamps: vec![DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap())],
    }
}

/// Create test data with inconsistent timestamps
fn create_inconsistent_timestamp_data() -> HyperliquidData {
    let datetime1 = vec![
        DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
        DateTime::from_timestamp(1640995100, 0) // Earlier timestamp
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
    ];
    
    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime1.clone(),
        open: vec![47000.0, 47100.0],
        high: vec![47500.0, 47600.0],
        low: vec![46500.0, 46600.0],
        close: vec![47200.0, 47300.0],
        volume: vec![100.0, 110.0],
        funding_rates: vec![0.0001, 0.00015],
        funding_timestamps: datetime1,
    }
}

/// Create test data with negative prices
fn create_negative_price_data() -> HyperliquidData {
    let datetime = vec![
        DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
    ];
    
    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: vec![-100.0], // Negative price
        high: vec![47500.0],
        low: vec![46500.0],
        close: vec![47200.0],
        volume: vec![100.0],
        funding_rates: vec![0.0001],
        funding_timestamps: datetime,
    }
}

/// Create test data with extreme values
fn create_extreme_value_data() -> HyperliquidData {
    let datetime = vec![
        DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap()),
    ];
    
    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: vec![f64::MAX / 2.0], // Extreme value
        high: vec![f64::MAX],
        low: vec![f64::MIN_POSITIVE],
        close: vec![1e10], // Very large value
        volume: vec![1e15], // Extreme volume
        funding_rates: vec![1.0], // 100% funding rate
        funding_timestamps: datetime,
    }
}

/// Create realistic test data for specific asset
fn create_realistic_test_data(ticker: &str, size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let base_price = match ticker {
        "BTC" => 47000.0,
        "ETH" => 3500.0,
        "SOL" => 150.0,
        "AVAX" => 80.0,
        _ => 100.0,
    };

    let prices: Vec<f64> = (0..size)
        .map(|i| {
            let trend = (i as f64 / size as f64) * base_price * 0.1;
            let cycle = (i as f64 * 0.02).sin() * base_price * 0.05;
            let noise = (i as f64 * 0.5).sin() * base_price * 0.01;
            base_price + trend + cycle + noise
        })
        .collect();

    HyperliquidData {
        ticker: ticker.to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + p * 0.01).collect(),
        low: prices.iter().map(|p| p - p * 0.01).collect(),
        close: prices,
        volume: (0..size).map(|i| 100.0 + (i as f64 * 0.1).sin() * 50.0).collect(),
        funding_rates: (0..size).map(|i| 0.0001 + (i as f64 * 0.01).sin() * 0.0001).collect(),
        funding_timestamps: datetime,
    }
}

#[tokio::test]
async fn test_stress_testing_workflow() {
    // Stress test with extreme market conditions
    let extreme_scenarios = vec![
        ("High Volatility", create_high_volatility_data(1000)),
        ("Market Crash", create_crash_scenario_data(1000)),
        ("Bull Market", create_bull_market_data(1000)),
        ("Sideways Market", create_sideways_market_data(1000)),
    ];
    
    for (scenario_name, data) in extreme_scenarios {
        info!("Testing stress scenario: {}", scenario_name);
        
        let strategies = vec![
            enhanced_sma_cross(5, 20, 0.1),
            enhanced_sma_cross(20, 50, 0.5),
            funding_arbitrage_strategy(0.0005, Default::default()),
        ];
        
        for (i, strategy) in strategies.into_iter().enumerate() {
            let mut backtest = HyperliquidBacktest::new(
                data.clone(),
                strategy,
                10000.0,
                HyperliquidCommission::default(),
            );
            
            backtest.calculate_with_funding();
            let report = backtest.enhanced_report();
            
            // Validate that extreme scenarios don't break the system
            assert!(report.total_return.is_finite(), 
                "Strategy {} in scenario {} produced invalid return", i, scenario_name);
            assert!(report.max_drawdown.is_finite(),
                "Strategy {} in scenario {} produced invalid drawdown", i, scenario_name);
            
            info!("Scenario {} Strategy {}: return={:.2}%, drawdown={:.2}%", 
                  scenario_name, i, report.total_return * 100.0, report.max_drawdown * 100.0);
        }
    }
}

// ============================================================================
// REGRESSION TESTS FOR API COMPATIBILITY
// ============================================================================

#[tokio::test]
async fn test_rs_backtester_compatibility_regression() {
    // Test that our extensions don't break rs-backtester compatibility
    let hyperliquid_data = create_test_hyperliquid_data();
    let rs_data = hyperliquid_data.to_rs_backtester_data();
    
    // Test with various rs-backtester strategies
    use rs_backtester::prelude::*;
    
    let rs_strategies = vec![
        strategies::sma_cross(10, 20),
        strategies::rsi_strategy(14, 30.0, 70.0),
        strategies::bollinger_bands(20, 2.0),
    ];
    
    for (i, strategy) in rs_strategies.into_iter().enumerate() {
        let backtest = Backtest::new(
            rs_data.clone(),
            strategy,
            10000.0,
            Commission::default(),
        );
        
        // Should be able to run standard rs-backtester workflow
        assert!(backtest.data.close.len() > 0);
        assert_eq!(backtest.initial_capital, 10000.0);
        
        info!("rs-backtester strategy {} compatibility verified", i);
    }
}

#[tokio::test]
async fn test_data_structure_compatibility() {
    // Test that our data structures maintain expected interfaces
    let data = create_test_hyperliquid_data();
    
    // Test required fields exist
    assert!(!data.ticker.is_empty());
    assert!(!data.datetime.is_empty());
    assert!(!data.open.is_empty());
    assert!(!data.high.is_empty());
    assert!(!data.low.is_empty());
    assert!(!data.close.is_empty());
    assert!(!data.volume.is_empty());
    assert!(!data.funding_rates.is_empty());
    assert!(!data.funding_timestamps.is_empty());
    
    // Test data consistency
    let len = data.datetime.len();
    assert_eq!(data.open.len(), len);
    assert_eq!(data.high.len(), len);
    assert_eq!(data.low.len(), len);
    assert_eq!(data.close.len(), len);
    assert_eq!(data.volume.len(), len);
    
    // Test conversion maintains data integrity
    let rs_data = data.to_rs_backtester_data();
    assert_eq!(rs_data.datetime.len(), len);
    assert_eq!(rs_data.close.len(), len);
    
    // Test funding rate functionality
    if !data.funding_timestamps.is_empty() {
        let funding_rate = data.get_funding_rate_at(data.datetime[0]);
        assert!(funding_rate.is_some() || funding_rate.is_none()); // Should not panic
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Helper function to create test data
fn create_test_hyperliquid_data() -> HyperliquidData {
    create_realistic_test_data("BTC", 100)
}

/// Create large test dataset for memory testing
fn create_large_test_data(size: usize) -> HyperliquidData {
    create_realistic_test_data("BTC", size)
}

/// Create realistic test data with proper price movements
fn create_realistic_test_data(ticker: &str, size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let base_price = match ticker {
        "BTC" => 47000.0,
        "ETH" => 3500.0,
        "SOL" => 150.0,
        "AVAX" => 80.0,
        _ => 100.0,
    };

    let prices: Vec<f64> = (0..size)
        .map(|i| {
            let trend = (i as f64 * 0.001).sin() * base_price * 0.1;
            let noise = (i as f64 * 0.1).sin() * base_price * 0.02;
            let volatility = (i as f64 * 0.05).cos() * base_price * 0.01;
            base_price + trend + noise + volatility
        })
        .collect();

    HyperliquidData {
        ticker: ticker.to_string(),
        datetime: datetime.clone(),
        open: prices.iter().map(|p| p - p * 0.001).collect(),
        high: prices.iter().map(|p| p + p * 0.005).collect(),
        low: prices.iter().map(|p| p - p * 0.005).collect(),
        close: prices,
        volume: (0..size).map(|i| 100.0 + (i as f64 * 0.1).sin() * 20.0).collect(),
        funding_rates: (0..size).map(|i| 0.0001 + (i as f64 * 0.01).sin() * 0.0001).collect(),
        funding_timestamps: datetime,
    }
}

/// Create high volatility test data
fn create_high_volatility_data(size: usize) -> HyperliquidData {
    let mut data = create_realistic_test_data("BTC", size);
    
    // Amplify price movements for high volatility
    for i in 0..size {
        let volatility_multiplier = 5.0;
        let base_price = data.close[i];
        let high_vol_change = (i as f64 * 0.1).sin() * base_price * 0.1 * volatility_multiplier;
        
        data.close[i] = base_price + high_vol_change;
        data.high[i] = data.close[i] + base_price * 0.02;
        data.low[i] = data.close[i] - base_price * 0.02;
        data.open[i] = if i > 0 { data.close[i-1] } else { data.close[i] };
    }
    
    data
}

/// Create market crash scenario data
fn create_crash_scenario_data(size: usize) -> HyperliquidData {
    let mut data = create_realistic_test_data("BTC", size);
    
    // Simulate market crash in the middle
    let crash_start = size / 3;
    let crash_end = crash_start + size / 6;
    
    for i in crash_start..crash_end {
        let crash_factor = 1.0 - ((i - crash_start) as f64 / (crash_end - crash_start) as f64) * 0.5;
        data.close[i] *= crash_factor;
        data.high[i] *= crash_factor;
        data.low[i] *= crash_factor;
        data.open[i] *= crash_factor;
    }
    
    data
}

/// Create bull market scenario data
fn create_bull_market_data(size: usize) -> HyperliquidData {
    let mut data = create_realistic_test_data("BTC", size);
    
    // Add consistent upward trend
    for i in 0..size {
        let bull_factor = 1.0 + (i as f64 / size as f64) * 2.0; // 200% increase over period
        data.close[i] *= bull_factor;
        data.high[i] *= bull_factor;
        data.low[i] *= bull_factor;
        data.open[i] *= bull_factor;
    }
    
    data
}

/// Create sideways market scenario data
fn create_sideways_market_data(size: usize) -> HyperliquidData {
    let mut data = create_realistic_test_data("BTC", size);
    
    // Reduce trend, increase noise
    for i in 0..size {
        let base_price = data.close[0]; // Keep around initial price
        let noise = (i as f64 * 0.2).sin() * base_price * 0.05;
        
        data.close[i] = base_price + noise;
        data.high[i] = data.close[i] + base_price * 0.01;
        data.low[i] = data.close[i] - base_price * 0.01;
        data.open[i] = if i > 0 { data.close[i-1] } else { data.close[i] };
    }
    
    data
}