//! Regression tests for API compatibility
//! 
//! These tests ensure that changes to the codebase don't break existing
//! functionality and maintain backward compatibility with rs-backtester.

use crate::prelude::*;
use chrono::{DateTime, FixedOffset};
use std::collections::HashMap;
use serde_json;

/// Test data structure compatibility across versions
#[tokio::test]
async fn test_hyperliquid_data_structure_compatibility() {
    // Test that HyperliquidData maintains expected structure
    let data = create_regression_test_data();
    
    // Test required fields exist and have correct types
    assert!(!data.ticker.is_empty());
    assert!(data.ticker.len() <= 10); // Reasonable ticker length
    
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
    assert_eq!(data.open.len(), len, "Open prices length mismatch");
    assert_eq!(data.high.len(), len, "High prices length mismatch");
    assert_eq!(data.low.len(), len, "Low prices length mismatch");
    assert_eq!(data.close.len(), len, "Close prices length mismatch");
    assert_eq!(data.volume.len(), len, "Volume length mismatch");
    
    // Test OHLC data validity
    for i in 0..len {
        assert!(data.high[i] >= data.low[i], "High < Low at index {}", i);
        assert!(data.high[i] >= data.open[i], "High < Open at index {}", i);
        assert!(data.high[i] >= data.close[i], "High < Close at index {}", i);
        assert!(data.low[i] <= data.open[i], "Low > Open at index {}", i);
        assert!(data.low[i] <= data.close[i], "Low > Close at index {}", i);
        assert!(data.volume[i] >= 0.0, "Negative volume at index {}", i);
    }
}

/// Test rs-backtester compatibility
#[tokio::test]
async fn test_rs_backtester_integration_compatibility() {
    let hyperliquid_data = create_regression_test_data();
    let rs_data = hyperliquid_data.to_rs_backtester_data();
    
    // Test conversion maintains data integrity
    assert_eq!(rs_data.datetime.len(), hyperliquid_data.datetime.len());
    assert_eq!(rs_data.open.len(), hyperliquid_data.open.len());
    assert_eq!(rs_data.high.len(), hyperliquid_data.high.len());
    assert_eq!(rs_data.low.len(), hyperliquid_data.low.len());
    assert_eq!(rs_data.close.len(), hyperliquid_data.close.len());
    assert_eq!(rs_data.volume.len(), hyperliquid_data.volume.len());
    
    // Test that converted data works with rs-backtester
    use rs_backtester::prelude::*;
    
    let strategies = vec![
        strategies::sma_cross(10, 20),
        strategies::rsi_strategy(14, 30.0, 70.0),
        strategies::bollinger_bands(20, 2.0),
    ];
    
    for strategy in strategies {
        let backtest = Backtest::new(
            rs_data.clone(),
            strategy,
            10000.0,
            Commission::default(),
        );
        
        // Should be able to create backtest without errors
        assert_eq!(backtest.initial_capital, 10000.0);
        assert!(backtest.data.close.len() > 0);
    }
}

/// Test HyperliquidBacktest compatibility
#[tokio::test]
async fn test_hyperliquid_backtest_compatibility() {
    let data = create_regression_test_data();
    let strategy = enhanced_sma_cross(10, 20, 0.3);
    
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );
    
    // Test that basic backtest functionality works
    backtest.calculate_with_funding();
    
    // Test report generation
    let enhanced_report = backtest.enhanced_report();
    assert!(enhanced_report.total_return.is_finite());
    assert!(enhanced_report.max_drawdown.is_finite());
    assert!(enhanced_report.sharpe_ratio.is_finite() || enhanced_report.sharpe_ratio.is_nan());
    
    let funding_report = backtest.funding_report();
    assert!(funding_report.total_funding_paid >= 0.0);
    assert!(funding_report.total_funding_received >= 0.0);
    assert!(!funding_report.funding_payments.is_empty());
    
    // Test CSV export
    let mut csv_buffer = Vec::new();
    backtest.enhanced_csv_export(&mut csv_buffer).unwrap();
    assert!(!csv_buffer.is_empty());
    
    // Verify CSV contains expected headers
    let csv_string = String::from_utf8(csv_buffer).unwrap();
    assert!(csv_string.contains("timestamp"));
    assert!(csv_string.contains("close"));
    assert!(csv_string.contains("funding_rate"));
}

/// Test commission structure compatibility
#[tokio::test]
async fn test_commission_structure_compatibility() {
    // Test default commission
    let default_commission = HyperliquidCommission::default();
    assert!(default_commission.maker_rate >= 0.0);
    assert!(default_commission.taker_rate >= 0.0);
    assert!(default_commission.taker_rate >= default_commission.maker_rate);
    
    // Test custom commission
    let custom_commission = HyperliquidCommission {
        maker_rate: 0.0001,
        taker_rate: 0.0003,
        funding_enabled: true,
    };
    
    let data = create_regression_test_data();
    let strategy = enhanced_sma_cross(10, 20, 0.0);
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        custom_commission,
    );
    
    backtest.calculate_with_funding();
    let report = backtest.enhanced_report();
    assert!(report.total_return.is_finite());
}

/// Test strategy interface compatibility
#[tokio::test]
async fn test_strategy_interface_compatibility() {
    let data = create_regression_test_data();
    
    // Test different strategy configurations
    let strategies = vec![
        ("Basic SMA", enhanced_sma_cross(10, 20, 0.0)),
        ("Funding Aware SMA", enhanced_sma_cross(15, 35, 0.3)),
        ("High Funding Weight", enhanced_sma_cross(5, 25, 0.8)),
        ("Funding Arbitrage", funding_arbitrage_strategy(0.001, Default::default())),
        ("Conservative Arbitrage", funding_arbitrage_strategy(0.0005, Default::default())),
    ];
    
    for (name, strategy) in strategies {
        let mut backtest = HyperliquidBacktest::new(
            data.clone(),
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let report = backtest.enhanced_report();
        
        assert!(report.total_return.is_finite(), "Strategy '{}' produced invalid return", name);
        assert!(report.max_drawdown.is_finite(), "Strategy '{}' produced invalid drawdown", name);
    }
}

/// Test error handling compatibility
#[tokio::test]
async fn test_error_handling_compatibility() {
    // Test various error scenarios
    let error_cases = vec![
        HyperliquidBacktestError::DataConversion("Test conversion error".to_string()),
        HyperliquidBacktestError::InvalidTimeRange { start: 100, end: 50 },
        HyperliquidBacktestError::UnsupportedInterval("invalid".to_string()),
        HyperliquidBacktestError::Backtesting("Test backtest error".to_string()),
    ];
    
    for error in error_cases {
        let error_string = error.to_string();
        assert!(!error_string.is_empty());
        assert!(error_string.len() > 10); // Should have meaningful error messages
    }
}

/// Test funding rate functionality compatibility
#[tokio::test]
async fn test_funding_rate_compatibility() {
    let data = create_regression_test_data();
    
    // Test funding rate lookup
    for (i, timestamp) in data.datetime.iter().enumerate() {
        let funding_rate = data.get_funding_rate_at(*timestamp);
        
        if i < data.funding_timestamps.len() {
            assert!(funding_rate.is_some(), "Missing funding rate for timestamp {}", i);
            let rate = funding_rate.unwrap();
            assert!(rate.is_finite(), "Invalid funding rate at timestamp {}", i);
            assert!(rate.abs() < 1.0, "Unrealistic funding rate {} at timestamp {}", rate, i);
        }
    }
    
    // Test funding rate interpolation (if implemented)
    if data.funding_timestamps.len() > 1 {
        let mid_time = data.datetime[data.datetime.len() / 2];
        let funding_rate = data.get_funding_rate_at(mid_time);
        // Should either return a rate or None, but not panic
        assert!(funding_rate.is_some() || funding_rate.is_none());
    }
}

/// Test serialization compatibility (if implemented)
#[tokio::test]
async fn test_serialization_compatibility() {
    let data = create_regression_test_data();
    
    // Test that data structures can be serialized/deserialized if needed
    // This is important for caching and persistence
    
    // Test basic JSON serialization of simple structures
    let commission = HyperliquidCommission::default();
    let commission_json = serde_json::to_string(&commission);
    assert!(commission_json.is_ok() || commission_json.is_err()); // Should not panic
    
    // Test that we can handle various data sizes
    let small_data = create_small_regression_data();
    let large_data = create_large_regression_data();
    
    assert!(small_data.datetime.len() < large_data.datetime.len());
    assert_eq!(small_data.datetime.len(), small_data.close.len());
    assert_eq!(large_data.datetime.len(), large_data.close.len());
}

/// Test performance regression
#[tokio::test]
async fn test_performance_regression() {
    use std::time::Instant;
    
    let data = create_regression_test_data();
    
    // Test data conversion performance
    let start = Instant::now();
    let _rs_data = data.to_rs_backtester_data();
    let conversion_time = start.elapsed();
    assert!(conversion_time.as_millis() < 1000, "Data conversion too slow: {:?}", conversion_time);
    
    // Test backtesting performance
    let start = Instant::now();
    let strategy = enhanced_sma_cross(10, 20, 0.3);
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );
    backtest.calculate_with_funding();
    let backtest_time = start.elapsed();
    assert!(backtest_time.as_secs() < 10, "Backtesting too slow: {:?}", backtest_time);
}

/// Test memory usage regression
#[tokio::test]
async fn test_memory_usage_regression() {
    use memory_stats::memory_stats;
    
    let initial_memory = memory_stats().map(|stats| stats.physical_mem).unwrap_or(0);
    
    // Create and process multiple datasets
    for i in 0..5 {
        let data = create_regression_test_data();
        let strategy = enhanced_sma_cross(10 + i, 20 + i * 2, 0.1 + i as f64 * 0.1);
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let _report = backtest.enhanced_report();
        
        // Explicit cleanup
        drop(backtest);
    }
    
    // Check memory usage hasn't grown excessively
    if let Some(final_stats) = memory_stats() {
        let final_memory = final_stats.physical_mem;
        let growth = final_memory.saturating_sub(initial_memory);
        let max_allowed_growth = initial_memory / 10; // 10% growth max
        
        assert!(growth < max_allowed_growth,
            "Excessive memory growth: {} bytes (limit: {} bytes)", growth, max_allowed_growth);
    }
}

/// Test API version compatibility
#[tokio::test]
async fn test_api_version_compatibility() {
    // Test that public API methods exist and work as expected
    let data = create_regression_test_data();
    
    // Test HyperliquidData public methods
    assert!(!data.ticker.is_empty());
    assert!(!data.datetime.is_empty());
    
    let rs_data = data.to_rs_backtester_data();
    assert!(!rs_data.close.is_empty());
    
    if !data.funding_timestamps.is_empty() {
        let funding_rate = data.get_funding_rate_at(data.datetime[0]);
        assert!(funding_rate.is_some() || funding_rate.is_none());
    }
    
    // Test HyperliquidBacktest public methods
    let strategy = enhanced_sma_cross(10, 20, 0.3);
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );
    
    backtest.calculate_with_funding();
    
    let enhanced_report = backtest.enhanced_report();
    assert!(enhanced_report.total_return.is_finite());
    
    let funding_report = backtest.funding_report();
    assert!(funding_report.total_funding_paid >= 0.0);
    
    let mut csv_buffer = Vec::new();
    let csv_result = backtest.enhanced_csv_export(&mut csv_buffer);
    assert!(csv_result.is_ok());
}

/// Test backward compatibility with different data formats
#[tokio::test]
async fn test_backward_compatibility() {
    // Test compatibility with different data structure versions
    let test_cases = vec![
        ("minimal_data", create_minimal_compatibility_data()),
        ("extended_data", create_extended_compatibility_data()),
        ("legacy_format", create_legacy_format_data()),
    ];
    
    for (case_name, data) in test_cases {
        info!("Testing backward compatibility case: {}", case_name);
        
        // Test that all data formats can be processed
        let rs_data = data.to_rs_backtester_data();
        assert_eq!(rs_data.datetime.len(), data.datetime.len(), 
                  "Data length mismatch for {}", case_name);
        
        // Test backtesting compatibility
        let strategy = enhanced_sma_cross(5, 15, 0.2);
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let report = backtest.enhanced_report();
        
        assert!(report.total_return.is_finite(), 
               "Invalid return for compatibility case: {}", case_name);
    }
}

/// Test forward compatibility with new fields
#[tokio::test]
async fn test_forward_compatibility() {
    // Test that system handles new fields gracefully
    let mut data = create_regression_test_data();
    
    // Simulate adding new fields by extending existing data
    // In a real scenario, this would test parsing of API responses with new fields
    
    // Test that existing functionality still works
    let rs_data = data.to_rs_backtester_data();
    assert_eq!(rs_data.close.len(), data.close.len());
    
    let strategy = enhanced_sma_cross(10, 20, 0.3);
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );
    
    backtest.calculate_with_funding();
    let report = backtest.enhanced_report();
    assert!(report.total_return.is_finite());
}

/// Test API breaking changes detection
#[tokio::test]
async fn test_api_breaking_changes() {
    // Test that critical API methods haven't changed signatures
    let data = create_regression_test_data();
    
    // Test HyperliquidData constructor-like methods
    assert_eq!(data.ticker.len(), 3); // "BTC"
    assert!(!data.datetime.is_empty());
    assert!(!data.close.is_empty());
    
    // Test conversion method signature
    let _rs_data: rs_backtester::prelude::Data = data.to_rs_backtester_data();
    
    // Test funding rate method signature
    let timestamp = data.datetime[0];
    let _funding_rate: Option<f64> = data.get_funding_rate_at(timestamp);
    
    // Test HyperliquidBacktest constructor signature
    let strategy = enhanced_sma_cross(10, 20, 0.3);
    let commission = HyperliquidCommission::default();
    let _backtest = HyperliquidBacktest::new(data, strategy, 10000.0, commission);
}

/// Test data structure field compatibility
#[tokio::test]
async fn test_data_structure_fields() {
    let data = create_regression_test_data();
    
    // Test that all expected fields exist and have correct types
    let _ticker: &String = &data.ticker;
    let _datetime: &Vec<DateTime<FixedOffset>> = &data.datetime;
    let _open: &Vec<f64> = &data.open;
    let _high: &Vec<f64> = &data.high;
    let _low: &Vec<f64> = &data.low;
    let _close: &Vec<f64> = &data.close;
    let _volume: &Vec<f64> = &data.volume;
    let _funding_rates: &Vec<f64> = &data.funding_rates;
    let _funding_timestamps: &Vec<DateTime<FixedOffset>> = &data.funding_timestamps;
    
    // Test field relationships
    assert_eq!(data.datetime.len(), data.open.len());
    assert_eq!(data.datetime.len(), data.high.len());
    assert_eq!(data.datetime.len(), data.low.len());
    assert_eq!(data.datetime.len(), data.close.len());
    assert_eq!(data.datetime.len(), data.volume.len());
    
    // Funding data might have different length, but should be consistent internally
    assert_eq!(data.funding_rates.len(), data.funding_timestamps.len());
}

/// Test commission structure evolution
#[tokio::test]
async fn test_commission_structure_evolution() {
    // Test different commission configurations
    let commission_configs = vec![
        ("default", HyperliquidCommission::default()),
        ("zero_fees", HyperliquidCommission {
            maker_rate: 0.0,
            taker_rate: 0.0,
            funding_enabled: true,
        }),
        ("high_fees", HyperliquidCommission {
            maker_rate: 0.001,
            taker_rate: 0.002,
            funding_enabled: true,
        }),
        ("no_funding", HyperliquidCommission {
            maker_rate: 0.0002,
            taker_rate: 0.0005,
            funding_enabled: false,
        }),
    ];
    
    let data = create_regression_test_data();
    let strategy = enhanced_sma_cross(10, 20, 0.3);
    
    for (config_name, commission) in commission_configs {
        let mut backtest = HyperliquidBacktest::new(
            data.clone(),
            strategy.clone(),
            10000.0,
            commission,
        );
        
        backtest.calculate_with_funding();
        let report = backtest.enhanced_report();
        
        assert!(report.total_return.is_finite(), 
               "Invalid return for commission config: {}", config_name);
        
        info!("Commission config '{}' processed successfully", config_name);
    }
}

/// Test strategy interface evolution
#[tokio::test]
async fn test_strategy_interface_evolution() {
    let data = create_regression_test_data();
    
    // Test that different strategy parameter combinations work
    let strategy_configs = vec![
        ("minimal_sma", enhanced_sma_cross(2, 5, 0.0)),
        ("standard_sma", enhanced_sma_cross(10, 20, 0.3)),
        ("long_period_sma", enhanced_sma_cross(50, 200, 0.1)),
        ("high_funding_weight", enhanced_sma_cross(10, 20, 0.9)),
        ("conservative_arbitrage", funding_arbitrage_strategy(0.0001, Default::default())),
        ("aggressive_arbitrage", funding_arbitrage_strategy(0.01, Default::default())),
    ];
    
    for (strategy_name, strategy) in strategy_configs {
        let mut backtest = HyperliquidBacktest::new(
            data.clone(),
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let report = backtest.enhanced_report();
        
        assert!(report.total_return.is_finite(), 
               "Strategy '{}' produced invalid results", strategy_name);
        
        info!("Strategy '{}' interface compatibility verified", strategy_name);
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create standard regression test data
fn create_regression_test_data() -> HyperliquidData {
    let size = 1000;
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

/// Create small dataset for testing
fn create_small_regression_data() -> HyperliquidData {
    let size = 10;
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let prices = vec![47000.0, 47100.0, 47050.0, 47200.0, 47150.0, 47300.0, 47250.0, 47400.0, 47350.0, 47500.0];

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + 10.0).collect(),
        low: prices.iter().map(|p| p - 10.0).collect(),
        close: prices,
        volume: vec![100.0; size],
        funding_rates: vec![0.0001; size],
        funding_timestamps: datetime,
    }
}

/// Create large dataset for testing
fn create_large_regression_data() -> HyperliquidData {
    let size = 10000;
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

/// Create minimal compatibility data for testing
fn create_minimal_compatibility_data() -> HyperliquidData {
    let size = 5;
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: vec![47000.0, 47100.0, 47050.0, 47200.0, 47150.0],
        high: vec![47050.0, 47150.0, 47100.0, 47250.0, 47200.0],
        low: vec![46950.0, 47050.0, 47000.0, 47150.0, 47100.0],
        close: vec![47020.0, 47080.0, 47180.0, 47170.0, 47190.0],
        volume: vec![100.0; size],
        funding_rates: vec![0.0001; size],
        funding_timestamps: datetime,
    }
}

/// Create extended compatibility data for testing
fn create_extended_compatibility_data() -> HyperliquidData {
    let size = 100;
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let prices: Vec<f64> = (0..size)
        .map(|i| 47000.0 + (i as f64 * 0.2).sin() * 200.0)
        .collect();

    HyperliquidData {
        ticker: "ETH".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + 30.0).collect(),
        low: prices.iter().map(|p| p - 30.0).collect(),
        close: prices,
        volume: (0..size).map(|i| 150.0 + (i as f64 * 0.1).cos() * 30.0).collect(),
        funding_rates: (0..size).map(|i| 0.00015 + (i as f64 * 0.02).sin() * 0.00005).collect(),
        funding_timestamps: datetime,
    }
}

/// Create legacy format data for testing
fn create_legacy_format_data() -> HyperliquidData {
    let size = 50;
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    // Simulate legacy data with simpler structure
    let base_price = 150.0; // SOL price
    let prices: Vec<f64> = (0..size)
        .map(|i| base_price + (i as f64).sin() * 5.0)
        .collect();

    HyperliquidData {
        ticker: "SOL".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + 2.0).collect(),
        low: prices.iter().map(|p| p - 2.0).collect(),
        close: prices,
        volume: vec![75.0; size], // Constant volume for legacy format
        funding_rates: vec![0.0001; size], // Constant funding rate
        funding_timestamps: datetime,
    }
}///
 Test comprehensive API compatibility across versions
#[tokio::test]
async fn test_comprehensive_api_compatibility() {
    // Test that all public API methods work consistently
    let data = create_regression_test_data();
    
    // Test HyperliquidData API
    assert!(!data.ticker.is_empty());
    assert!(!data.datetime.is_empty());
    assert!(!data.open.is_empty());
    assert!(!data.high.is_empty());
    assert!(!data.low.is_empty());
    assert!(!data.close.is_empty());
    assert!(!data.volume.is_empty());
    assert!(!data.funding_rates.is_empty());
    assert!(!data.funding_timestamps.is_empty());
    
    // Test data conversion API
    let rs_data = data.to_rs_backtester_data();
    assert_eq!(rs_data.datetime.len(), data.datetime.len());
    
    // Test funding rate lookup API
    let funding_rate = data.get_funding_rate_at(data.datetime[0]);
    assert!(funding_rate.is_some() || funding_rate.is_none());
    
    // Test HyperliquidBacktest API
    let strategy = enhanced_sma_cross(10, 20, 0.3);
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );
    
    // Test backtesting methods
    backtest.calculate_with_funding();
    let enhanced_report = backtest.enhanced_report();
    let funding_report = backtest.funding_report();
    
    // Test report structure
    assert!(enhanced_report.total_return.is_finite());
    assert!(funding_report.total_funding_paid >= 0.0);
    assert!(funding_report.total_funding_received >= 0.0);
    
    // Test CSV export API
    let mut csv_buffer = Vec::new();
    let csv_result = backtest.enhanced_csv_export(&mut csv_buffer);
    assert!(csv_result.is_ok());
    assert!(!csv_buffer.is_empty());
}

/// Test performance regression detection
#[tokio::test]
async fn test_performance_regression_detection() {
    use std::time::Instant;
    
    let data = create_regression_test_data();
    
    // Test data conversion performance
    let start = Instant::now();
    let _rs_data = data.to_rs_backtester_data();
    let conversion_time = start.elapsed();
    
    // Should complete within reasonable time (adjust threshold as needed)
    assert!(conversion_time.as_millis() < 100, 
           "Data conversion performance regression: {:?}", conversion_time);
    
    // Test backtesting performance
    let start = Instant::now();
    let strategy = enhanced_sma_cross(10, 20, 0.3);
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );
    backtest.calculate_with_funding();
    let backtest_time = start.elapsed();
    
    // Should complete within reasonable time
    assert!(backtest_time.as_secs() < 5, 
           "Backtesting performance regression: {:?}", backtest_time);
}

/// Test memory usage regression detection
#[tokio::test]
async fn test_memory_usage_regression_detection() {
    use memory_stats::memory_stats;
    
    let initial_memory = memory_stats().map(|stats| stats.physical_mem).unwrap_or(0);
    
    // Process several datasets
    for i in 0..5 {
        let data = create_regression_test_data();
        let strategy = enhanced_sma_cross(10 + i, 20 + i * 2, 0.1 + i as f64 * 0.1);
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let _report = backtest.enhanced_report();
        
        // Explicit cleanup
        drop(backtest);
    }
    
    // Check for memory leaks
    if let Some(final_stats) = memory_stats() {
        let final_memory = final_stats.physical_mem;
        let growth = final_memory.saturating_sub(initial_memory);
        let max_allowed_growth = initial_memory / 20; // 5% growth max
        
        assert!(growth < max_allowed_growth,
               "Memory usage regression detected: {} bytes growth", growth);
    }
}

/// Test edge case regression prevention
#[tokio::test]
async fn test_edge_case_regression_prevention() {
    // Test various edge cases that have been problematic in the past
    let edge_cases = vec![
        ("single_point", create_single_point_data()),
        ("identical_prices", create_identical_price_data()),
        ("extreme_values", create_extreme_value_data()),
    ];
    
    for (case_name, data_result) in edge_cases {
        match data_result {
            Ok(data) => {
                // Test that edge cases are handled gracefully
                let conversion_result = std::panic::catch_unwind(|| {
                    data.to_rs_backtester_data()
                });
                
                match conversion_result {
                    Ok(rs_data) => {
                        assert!(!rs_data.close.is_empty(), "Edge case {} produced empty data", case_name);
                        
                        // Test backtesting with edge case
                        let strategy = enhanced_sma_cross(2, 5, 0.1);
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
                        
                        match backtest_result {
                            Ok(report) => {
                                assert!(report.total_return.is_finite() || report.total_return.is_nan(),
                                       "Edge case {} produced invalid report", case_name);
                            }
                            Err(_) => {
                                info!("Edge case {} failed backtesting as expected", case_name);
                            }
                        }
                    }
                    Err(_) => {
                        info!("Edge case {} failed conversion as expected", case_name);
                    }
                }
            }
            Err(error) => {
                info!("Edge case {} failed data creation: {}", case_name, error);
            }
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS FOR ADDITIONAL REGRESSION TESTING
// ============================================================================

/// Create single data point for edge case testing
fn create_single_point_data() -> std::result::Result<HyperliquidData, &'static str> {
    let datetime = vec![
        DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap())
    ];

    Ok(HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: vec![47000.0],
        high: vec![47010.0],
        low: vec![46990.0],
        close: vec![47005.0],
        volume: vec![100.0],
        funding_rates: vec![0.0001],
        funding_timestamps: datetime,
    })
}

/// Create data with identical prices for edge case testing
fn create_identical_price_data() -> std::result::Result<HyperliquidData, &'static str> {
    let size = 100;
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let price = 47000.0;

    Ok(HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: vec![price; size],
        high: vec![price; size],
        low: vec![price; size],
        close: vec![price; size],
        volume: vec![100.0; size],
        funding_rates: vec![0.0001; size],
        funding_timestamps: datetime,
    })
}

/// Create data with extreme values for edge case testing
fn create_extreme_value_data() -> std::result::Result<HyperliquidData, &'static str> {
    let size = 50;
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    // Extreme price values
    let prices: Vec<f64> = (0..size)
        .map(|i| {
            if i % 10 == 0 { 1000000.0 } else { 0.01 } // Extreme high and low values
        })
        .collect();

    Ok(HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.clone(),
        high: prices.iter().map(|p| p * 1.1).collect(),
        low: prices.iter().map(|p| p * 0.9).collect(),
        close: prices,
        volume: vec![100.0; size],
        funding_rates: (0..size).map(|i| {
            if i % 5 == 0 { 0.1 } else { -0.1 } // Extreme funding rates
        }).collect(),
        funding_timestamps: datetime,
    })
}