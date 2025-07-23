//! Tests for data conversion

use crate::data::HyperliquidData;
use crate::errors::Result;
use crate::tests::mock_data::generate_mock_data;
use chrono::{FixedOffset, TimeZone};
use std::collections::HashMap;

/// Test that HyperliquidData can be converted to rs-backtester Data and back
#[test]
fn test_data_conversion_roundtrip() {
    let symbol = "BTC";
    let hours = 24;
    let with_funding = true;
    
    let data = generate_mock_data(symbol, hours, with_funding, false);
    
    // Convert to rs-backtester Data
    let rs_data = data.to_rs_backtester_data();
    
    // Verify conversion preserves data
    assert_eq!(rs_data.ticker, data.symbol);
    assert_eq!(rs_data.datetime.len(), data.datetime.len());
    assert_eq!(rs_data.open.len(), data.open.len());
    assert_eq!(rs_data.high.len(), data.high.len());
    assert_eq!(rs_data.low.len(), data.low.len());
    assert_eq!(rs_data.close.len(), data.close.len());
    assert_eq!(rs_data.volume.len(), data.volume.len());
    
    // Verify data values are preserved
    for i in 0..data.len() {
        assert_eq!(rs_data.datetime[i], data.datetime[i]);
        assert_eq!(rs_data.open[i], data.open[i]);
        assert_eq!(rs_data.high[i], data.high[i]);
        assert_eq!(rs_data.low[i], data.low[i]);
        assert_eq!(rs_data.close[i], data.close[i]);
        assert_eq!(rs_data.volume[i], data.volume[i]);
    }
}

/// Test that string-to-float conversion works correctly
#[test]
fn test_string_to_float_conversion() {
    // Test various float values
    let test_cases = vec![
        (0.0, "0"),
        (1.0, "1"),
        (-1.0, "-1"),
        (3.14159, "3.14159"),
        (0.0001, "0.0001"),
        (-0.0001, "-0.0001"),
        (1000000.0, "1000000"),
    ];
    
    for (value, string) in test_cases {
        // Format with specified precision
        let formatted = format!("{}", value);
        
        // Parse back to float
        let parsed: f64 = formatted.parse().unwrap();
        
        // Check that the parsed value is close to the original
        assert!((parsed - value).abs() < 0.0001);
        
        // Parse the string directly
        let parsed_string: f64 = string.parse().unwrap();
        
        // Check that the parsed value is close to the expected
        assert!((parsed_string - value).abs() < 0.0001);
    }
}

/// Test that timestamp conversion works correctly
#[test]
fn test_timestamp_conversion() {
    // Test various timestamps
    let test_cases = vec![
        1000000000, // 2001-09-09
        1500000000, // 2017-07-14
        1600000000, // 2020-09-13
        1700000000, // 2023-11-15
    ];
    
    for timestamp in test_cases {
        let tz = FixedOffset::east_opt(0).unwrap();
        let dt = tz.timestamp_opt(timestamp, 0).unwrap();
        
        // Convert back to timestamp
        let converted_timestamp = dt.timestamp();
        
        // Should be the same
        assert_eq!(timestamp, converted_timestamp);
    }
}

/// Test that HyperliquidData validation works correctly
#[test]
fn test_data_validation() -> Result<()> {
    // Valid data should pass validation
    let valid_data = generate_mock_data("BTC", 24, true, false);
    assert!(valid_data.validate_all_data().is_ok());
    
    // Test various invalid data scenarios
    let invalid_data_map = crate::tests::mock_data::generate_invalid_data();
    
    // Empty data should pass validation (it's valid, just empty)
    let empty_data = &invalid_data_map["empty"];
    assert!(empty_data.validate_all_data().is_ok());
    
    // Mismatched array lengths should fail validation
    let mismatched_data = &invalid_data_map["mismatched_lengths"];
    assert!(mismatched_data.validate_all_data().is_err());
    
    // Invalid high/low should fail validation
    let invalid_high_low = &invalid_data_map["invalid_high_low"];
    assert!(invalid_high_low.validate_all_data().is_err());
    
    // Non-chronological timestamps should fail validation
    let non_chronological = &invalid_data_map["non_chronological"];
    assert!(non_chronological.validate_all_data().is_err());
    
    Ok(())
}

/// Test that fetch parameters validation works correctly
#[test]
fn test_fetch_parameters_validation() {
    // Valid parameters should pass validation
    let result = HyperliquidData::validate_fetch_parameters(
        "BTC", "1h", 1640995200, 1641081600
    );
    assert!(result.is_ok());
    
    // Empty coin should fail validation
    let result = HyperliquidData::validate_fetch_parameters(
        "", "1h", 1640995200, 1641081600
    );
    assert!(result.is_err());
    
    // Unsupported interval should fail validation
    let result = HyperliquidData::validate_fetch_parameters(
        "BTC", "2h", 1640995200, 1641081600
    );
    assert!(result.is_err());
    
    // Invalid time range should fail validation
    let result = HyperliquidData::validate_fetch_parameters(
        "BTC", "1h", 1641081600, 1640995200
    );
    assert!(result.is_err());
    
    // Future timestamps should fail validation
    let future_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() + 100000;
    
    let result = HyperliquidData::validate_fetch_parameters(
        "BTC", "1h", future_time, future_time + 3600
    );
    assert!(result.is_err());
    
    // Time range too large should fail validation
    let start_time = 1640995200;
    let max_range = crate::data::HyperliquidDataFetcher::max_time_range_for_interval("1h");
    let end_time = start_time + max_range + 3600;
    
    let result = HyperliquidData::validate_fetch_parameters(
        "BTC", "1h", start_time, end_time
    );
    assert!(result.is_err());
}

/// Test that HyperliquidData constructors work correctly
#[test]
fn test_hyperliquid_data_constructors() -> Result<()> {
    let symbol = "BTC";
    let datetime = vec![
        FixedOffset::east_opt(0).unwrap().timestamp_opt(1640995200, 0).unwrap(),
        FixedOffset::east_opt(0).unwrap().timestamp_opt(1640998800, 0).unwrap(),
    ];
    let open = vec![100.0, 101.0];
    let high = vec![105.0, 106.0];
    let low = vec![95.0, 96.0];
    let close = vec![102.0, 103.0];
    let volume = vec![1000.0, 1100.0];
    let funding_rates = vec![0.0001, -0.0001];
    
    // Test with_ohlc_data constructor
    let data1 = HyperliquidData::with_ohlc_data(
        symbol.to_string(),
        datetime.clone(),
        open.clone(),
        high.clone(),
        low.clone(),
        close.clone(),
        volume.clone(),
    )?;
    
    assert_eq!(data1.symbol, symbol);
    assert_eq!(data1.datetime, datetime);
    assert_eq!(data1.open, open);
    assert_eq!(data1.high, high);
    assert_eq!(data1.low, low);
    assert_eq!(data1.close, close);
    assert_eq!(data1.volume, volume);
    assert!(data1.funding_rates.iter().all(|&r| r.is_nan()));
    
    // Test with_ohlc_and_funding_data constructor
    let data2 = HyperliquidData::with_ohlc_and_funding_data(
        symbol.to_string(),
        datetime.clone(),
        open.clone(),
        high.clone(),
        low.clone(),
        close.clone(),
        volume.clone(),
        funding_rates.clone(),
    )?;
    
    assert_eq!(data2.symbol, symbol);
    assert_eq!(data2.datetime, datetime);
    assert_eq!(data2.open, open);
    assert_eq!(data2.high, high);
    assert_eq!(data2.low, low);
    assert_eq!(data2.close, close);
    assert_eq!(data2.volume, volume);
    assert_eq!(data2.funding_rates, funding_rates);
    
    // Test constructor with mismatched array lengths
    let mut short_open = open.clone();
    short_open.pop();
    
    let result = HyperliquidData::with_ohlc_data(
        symbol.to_string(),
        datetime.clone(),
        short_open,
        high.clone(),
        low.clone(),
        close.clone(),
        volume.clone(),
    );
    
    assert!(result.is_err());
    
    Ok(())
}

/// Test that funding rate statistics calculation works correctly
#[test]
fn test_funding_statistics_calculation() {
    // Create data with known funding rates
    let mut data = generate_mock_data("BTC", 24, false, false);
    
    // Set specific funding rates for testing
    data.funding_rates = vec![
        0.0001, 0.0002, 0.0003, -0.0001, -0.0002,
        0.0001, 0.0002, 0.0003, -0.0001, -0.0002,
        0.0001, 0.0002, 0.0003, -0.0001, -0.0002,
        0.0001, 0.0002, 0.0003, -0.0001, -0.0002,
        0.0001, 0.0002, 0.0003, -0.0001,
    ];
    
    // Calculate statistics
    let stats = data.calculate_funding_statistics();
    
    // Verify statistics
    assert_eq!(stats.total_periods, 24);
    assert_eq!(stats.positive_periods, 16);
    assert_eq!(stats.negative_periods, 8);
    
    // Calculate expected average
    let expected_avg = data.funding_rates.iter().sum::<f64>() / 24.0;
    assert!((stats.average_rate - expected_avg).abs() < 0.0000001);
    
    // Min and max should match the actual min and max
    assert_eq!(stats.min_rate, -0.0002);
    assert_eq!(stats.max_rate, 0.0003);
    
    // Test with empty data
    let empty_data = HyperliquidData {
        symbol: "BTC".to_string(),
        datetime: Vec::new(),
        open: Vec::new(),
        high: Vec::new(),
        low: Vec::new(),
        close: Vec::new(),
        volume: Vec::new(),
        funding_rates: Vec::new(),
    };
    
    let empty_stats = empty_data.calculate_funding_statistics();
    assert_eq!(empty_stats.total_periods, 0);
    assert_eq!(empty_stats.positive_periods, 0);
    assert_eq!(empty_stats.negative_periods, 0);
    assert_eq!(empty_stats.average_rate, 0.0);
    assert_eq!(empty_stats.volatility, 0.0);
}

/// Test that get_funding_rate_at works correctly
#[test]
fn test_get_funding_rate_at() {
    // Create data with known funding rates
    let mut data = generate_mock_data("BTC", 24, false, false);
    
    // Set specific funding rates for testing
    for i in 0..24 {
        if i % 8 == 0 {
            data.funding_rates[i] = 0.0001 * (i as f64 / 8.0 + 1.0);
        } else {
            data.funding_rates[i] = f64::NAN;
        }
    }
    
    // Test exact matches
    for i in 0..3 {
        let timestamp = data.datetime[i * 8];
        let rate = data.get_funding_rate_at(timestamp);
        assert!(rate.is_some());
        assert_eq!(rate.unwrap(), 0.0001 * (i as f64 + 1.0));
    }
    
    // Test non-funding timestamps
    for i in 1..8 {
        let timestamp = data.datetime[i];
        let rate = data.get_funding_rate_at(timestamp);
        assert!(rate.is_none());
    }
    
    // Test timestamp not in the data
    let unknown_timestamp = FixedOffset::east_opt(0).unwrap()
        .timestamp_opt(1650000000, 0).unwrap();
    let rate = data.get_funding_rate_at(unknown_timestamp);
    assert!(rate.is_none());
}

/// Test that popular trading pairs functions work correctly
#[test]
fn test_popular_trading_pairs() {
    let pairs = HyperliquidData::popular_trading_pairs();
    
    // Check that we have some popular pairs
    assert!(!pairs.is_empty());
    
    // Check that BTC and ETH are included
    assert!(pairs.contains(&"BTC"));
    assert!(pairs.contains(&"ETH"));
    
    // Test is_popular_pair function
    assert!(HyperliquidData::is_popular_pair("BTC"));
    assert!(HyperliquidData::is_popular_pair("ETH"));
    assert!(!HyperliquidData::is_popular_pair("UNKNOWN"));
}

/// Test that supported intervals functions work correctly
#[test]
fn test_supported_intervals() {
    let intervals = crate::data::HyperliquidDataFetcher::supported_intervals();
    
    // Check that we have some supported intervals
    assert!(!intervals.is_empty());
    
    // Check that common intervals are included
    assert!(intervals.contains(&"1m"));
    assert!(intervals.contains(&"1h"));
    assert!(intervals.contains(&"1d"));
    
    // Test is_interval_supported function
    assert!(crate::data::HyperliquidDataFetcher::is_interval_supported("1m"));
    assert!(crate::data::HyperliquidDataFetcher::is_interval_supported("1h"));
    assert!(crate::data::HyperliquidDataFetcher::is_interval_supported("1d"));
    assert!(!crate::data::HyperliquidDataFetcher::is_interval_supported("2h"));
}

/// Test that max_time_range_for_interval works correctly
#[test]
fn test_max_time_range_for_interval() {
    // Check that different intervals have different max ranges
    let range_1m = crate::data::HyperliquidDataFetcher::max_time_range_for_interval("1m");
    let range_1h = crate::data::HyperliquidDataFetcher::max_time_range_for_interval("1h");
    let range_1d = crate::data::HyperliquidDataFetcher::max_time_range_for_interval("1d");
    
    // Higher granularity should have shorter max range
    assert!(range_1m < range_1h);
    assert!(range_1h < range_1d);
    
    // Check default for unknown interval
    let range_unknown = crate::data::HyperliquidDataFetcher::max_time_range_for_interval("unknown");
    assert_eq!(range_unknown, 365 * 24 * 3600); // Default to 1 year
}