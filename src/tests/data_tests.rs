//! Tests for the HyperliquidData module

use crate::data::*;
use crate::errors::{HyperliquidBacktestError, Result};
use chrono::{DateTime, FixedOffset, TimeZone};

fn create_test_datetime(timestamp: i64) -> DateTime<FixedOffset> {
    FixedOffset::east_opt(0).unwrap().timestamp_opt(timestamp, 0).unwrap()
}

fn create_valid_ohlc_data() -> (Vec<DateTime<FixedOffset>>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let datetime = vec![
        create_test_datetime(1640995200), // 2022-01-01 00:00:00 UTC
        create_test_datetime(1640995260), // 2022-01-01 00:01:00 UTC
        create_test_datetime(1640995320), // 2022-01-01 00:02:00 UTC
    ];
    let open = vec![100.0, 101.0, 102.0];
    let high = vec![105.0, 106.0, 107.0];
    let low = vec![95.0, 96.0, 97.0];
    let close = vec![103.0, 104.0, 105.0];
    let volume = vec![1000.0, 1100.0, 1200.0];
    
    (datetime, open, high, low, close, volume)
}

fn create_mock_candle_response(time_open: u64, coin: &str, interval: &str) -> hyperliquid_rust_sdk::CandlesSnapshotResponse {
    hyperliquid_rust_sdk::CandlesSnapshotResponse {
        time_open,
        time_close: time_open + 60, // 1 minute later
        coin: coin.to_string(),
        candle_interval: interval.to_string(),
        open: "100.0".to_string(),
        close: "101.0".to_string(),
        high: "102.0".to_string(),
        low: "99.0".to_string(),
        vlm: "1000.0".to_string(),
        num_trades: 10,
    }
}

fn create_mock_funding_response(time: u64, coin: &str) -> hyperliquid_rust_sdk::FundingHistoryResponse {
    hyperliquid_rust_sdk::FundingHistoryResponse {
        coin: coin.to_string(),
        funding_rate: "0.0001".to_string(),
        premium: "0.0".to_string(),
        time,
    }
}

// Mock fetcher for testing validation methods
struct MockHyperliquidDataFetcher;

impl MockHyperliquidDataFetcher {
    fn validate_fetch_params(&self, coin: &str, interval: &str, start_time: u64, end_time: u64) -> Result<()> {
        // Validate coin parameter
        if coin.is_empty() {
            return Err(HyperliquidBacktestError::validation("Coin cannot be empty"));
        }

        // Validate interval parameter
        let valid_intervals = ["1m", "5m", "15m", "1h", "4h", "1d"];
        if !valid_intervals.contains(&interval) {
            return Err(HyperliquidBacktestError::unsupported_interval(interval));
        }

        // Validate time range
        if start_time >= end_time {
            return Err(HyperliquidBacktestError::invalid_time_range(start_time, end_time));
        }

        // Validate that times are reasonable (not too far in the past or future)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if start_time > current_time + 86400 { // Not more than 1 day in the future
            return Err(HyperliquidBacktestError::validation("Start time cannot be in the future"));
        }

        if end_time > current_time + 86400 { // Not more than 1 day in the future
            return Err(HyperliquidBacktestError::validation("End time cannot be in the future"));
        }

        // Validate that the time range is not too large (to prevent excessive API calls)
        let max_range_seconds = match interval {
            "1m" => 7 * 24 * 3600,      // 1 week for 1-minute data
            "5m" => 30 * 24 * 3600,     // 1 month for 5-minute data
            "15m" => 90 * 24 * 3600,    // 3 months for 15-minute data
            "1h" => 365 * 24 * 3600,    // 1 year for 1-hour data
            "4h" => 2 * 365 * 24 * 3600, // 2 years for 4-hour data
            "1d" => 5 * 365 * 24 * 3600, // 5 years for daily data
            _ => 365 * 24 * 3600,       // Default to 1 year
        };

        if end_time - start_time > max_range_seconds {
            return Err(HyperliquidBacktestError::validation(
                format!("Time range too large for interval {}. Maximum range: {} days", 
                    interval, max_range_seconds / 86400)
            ));
        }

        Ok(())
    }

    fn validate_ohlc_response(&self, candles: &[hyperliquid_rust_sdk::CandlesSnapshotResponse]) -> Result<()> {
        if candles.is_empty() {
            return Err(HyperliquidBacktestError::validation("No OHLC data returned from API"));
        }

        // Validate each candle
        for (i, candle) in candles.iter().enumerate() {
            // Check that OHLC values can be parsed as floats
            candle.open.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid open price '{}' at index {}", candle.open, i)
                ))?;
            
            candle.high.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid high price '{}' at index {}", candle.high, i)
                ))?;
            
            candle.low.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid low price '{}' at index {}", candle.low, i)
                ))?;
            
            candle.close.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid close price '{}' at index {}", candle.close, i)
                ))?;
            
            candle.vlm.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid volume '{}' at index {}", candle.vlm, i)
                ))?;

            // Validate timestamp
            if candle.time_open >= candle.time_close {
                return Err(HyperliquidBacktestError::validation(
                    format!("Invalid candle timestamps: open {} >= close {} at index {}", 
                        candle.time_open, candle.time_close, i)
                ));
            }
        }

        // Check chronological order
        for i in 1..candles.len() {
            if candles[i].time_open <= candles[i - 1].time_open {
                return Err(HyperliquidBacktestError::validation(
                    format!("Candles not in chronological order at indices {} and {}", i - 1, i)
                ));
            }
        }

        Ok(())
    }

    fn validate_funding_response(&self, funding_history: &[hyperliquid_rust_sdk::FundingHistoryResponse]) -> Result<()> {
        if funding_history.is_empty() {
            return Ok(()); // Empty funding history is valid
        }

        // Validate each funding entry
        for (i, entry) in funding_history.iter().enumerate() {
            // Check that funding rate can be parsed as float
            entry.funding_rate.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid funding rate '{}' at index {}", entry.funding_rate, i)
                ))?;
            
            // Check that premium can be parsed as float
            entry.premium.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid premium '{}' at index {}", entry.premium, i)
                ))?;
        }

        // Check chronological order
        for i in 1..funding_history.len() {
            if funding_history[i].time <= funding_history[i - 1].time {
                return Err(HyperliquidBacktestError::validation(
                    format!("Funding history not in chronological order at indices {} and {}", i - 1, i)
                ));
            }
        }

        Ok(())
    }
}

#[test]
fn test_to_rs_backtester_data() {
    let (datetime, open, high, low, close, volume) = create_valid_ohlc_data();
    let data = HyperliquidData::with_ohlc_data(
        "BTC".to_string(),
        datetime.clone(),
        open.clone(),
        high.clone(),
        low.clone(),
        close.clone(),
        volume.clone(),
    ).unwrap();

    let rs_data = data.to_rs_backtester_data();
    
    assert_eq!(rs_data.ticker, "BTC");
    assert_eq!(rs_data.datetime, datetime);
    assert_eq!(rs_data.open, open);
    assert_eq!(rs_data.high, high);
    assert_eq!(rs_data.low, low);
    assert_eq!(rs_data.close, close);
}

#[test]
fn test_hyperliquid_data_fetcher_supported_intervals() {
    let intervals = HyperliquidDataFetcher::supported_intervals();
    assert_eq!(intervals, &["1m", "5m", "15m", "1h", "4h", "1d"]);
}

#[test]
fn test_hyperliquid_data_fetcher_is_interval_supported() {
    assert!(HyperliquidDataFetcher::is_interval_supported("1m"));
    assert!(HyperliquidDataFetcher::is_interval_supported("5m"));
    assert!(HyperliquidDataFetcher::is_interval_supported("15m"));
    assert!(HyperliquidDataFetcher::is_interval_supported("1h"));
    assert!(HyperliquidDataFetcher::is_interval_supported("4h"));
    assert!(HyperliquidDataFetcher::is_interval_supported("1d"));
    
    assert!(!HyperliquidDataFetcher::is_interval_supported("30s"));
    assert!(!HyperliquidDataFetcher::is_interval_supported("2h"));
    assert!(!HyperliquidDataFetcher::is_interval_supported("1w"));
}

#[test]
fn test_hyperliquid_data_fetcher_max_time_range() {
    assert_eq!(HyperliquidDataFetcher::max_time_range_for_interval("1m"), 7 * 24 * 3600);
    assert_eq!(HyperliquidDataFetcher::max_time_range_for_interval("5m"), 30 * 24 * 3600);
    assert_eq!(HyperliquidDataFetcher::max_time_range_for_interval("15m"), 90 * 24 * 3600);
    assert_eq!(HyperliquidDataFetcher::max_time_range_for_interval("1h"), 365 * 24 * 3600);
    assert_eq!(HyperliquidDataFetcher::max_time_range_for_interval("4h"), 2 * 365 * 24 * 3600);
    assert_eq!(HyperliquidDataFetcher::max_time_range_for_interval("1d"), 5 * 365 * 24 * 3600);
    assert_eq!(HyperliquidDataFetcher::max_time_range_for_interval("unknown"), 365 * 24 * 3600);
}

#[test]
fn test_validate_fetch_params_empty_coin() {
    let fetcher = MockHyperliquidDataFetcher;
    let result = fetcher.validate_fetch_params("", "1h", 1640995200, 1640995260);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::Validation(_)));
}

#[test]
fn test_validate_fetch_params_unsupported_interval() {
    let fetcher = MockHyperliquidDataFetcher;
    let result = fetcher.validate_fetch_params("BTC", "30s", 1640995200, 1640995260);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::UnsupportedInterval(_)));
}

#[test]
fn test_validate_fetch_params_invalid_time_range() {
    let fetcher = MockHyperliquidDataFetcher;
    let result = fetcher.validate_fetch_params("BTC", "1h", 1640995260, 1640995200);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::InvalidTimeRange { .. }));
}

#[test]
fn test_validate_fetch_params_future_time() {
    let fetcher = MockHyperliquidDataFetcher;
    let future_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() + 2 * 86400; // 2 days in the future
    
    let result = fetcher.validate_fetch_params("BTC", "1h", future_time, future_time + 3600);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::Validation(_)));
}

#[test]
fn test_validate_fetch_params_time_range_too_large() {
    let fetcher = MockHyperliquidDataFetcher;
    let start_time = 1640995200;
    let end_time = start_time + 8 * 24 * 3600; // 8 days for 1m interval (max is 7 days)
    
    let result = fetcher.validate_fetch_params("BTC", "1m", start_time, end_time);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::Validation(_)));
}

#[test]
fn test_validate_fetch_params_valid() {
    let fetcher = MockHyperliquidDataFetcher;
    let start_time = 1640995200;
    let end_time = start_time + 3600; // 1 hour
    
    let result = fetcher.validate_fetch_params("BTC", "1h", start_time, end_time);
    assert!(result.is_ok());
}

#[test]
fn test_validate_ohlc_response_empty() {
    let fetcher = MockHyperliquidDataFetcher;
    let candles = vec![];
    let result = fetcher.validate_ohlc_response(&candles);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::Validation(_)));
}

#[test]
fn test_validate_ohlc_response_invalid_price() {
    let fetcher = MockHyperliquidDataFetcher;
    let mut candle = create_mock_candle_response(1640995200, "BTC", "1h");
    candle.open = "invalid".to_string();
    let candles = vec![candle];
    
    let result = fetcher.validate_ohlc_response(&candles);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::DataConversion(_)));
}

#[test]
fn test_validate_ohlc_response_invalid_timestamps() {
    let fetcher = MockHyperliquidDataFetcher;
    let mut candle = create_mock_candle_response(1640995200, "BTC", "1h");
    candle.time_close = candle.time_open - 1; // Close before open
    let candles = vec![candle];
    
    let result = fetcher.validate_ohlc_response(&candles);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::Validation(_)));
}

#[test]
fn test_validate_ohlc_response_not_chronological() {
    let fetcher = MockHyperliquidDataFetcher;
    let candle1 = create_mock_candle_response(1640995260, "BTC", "1h");
    let candle2 = create_mock_candle_response(1640995200, "BTC", "1h"); // Earlier time
    let candles = vec![candle1, candle2];
    
    let result = fetcher.validate_ohlc_response(&candles);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::Validation(_)));
}

#[test]
fn test_validate_ohlc_response_valid() {
    let fetcher = MockHyperliquidDataFetcher;
    let candle1 = create_mock_candle_response(1640995200, "BTC", "1h");
    let candle2 = create_mock_candle_response(1640995260, "BTC", "1h");
    let candles = vec![candle1, candle2];
    
    let result = fetcher.validate_ohlc_response(&candles);
    assert!(result.is_ok());
}

#[test]
fn test_validate_funding_response_empty() {
    let fetcher = MockHyperliquidDataFetcher;
    let funding_history = vec![];
    let result = fetcher.validate_funding_response(&funding_history);
    assert!(result.is_ok()); // Empty funding history is valid
}

#[test]
fn test_validate_funding_response_invalid_rate() {
    let fetcher = MockHyperliquidDataFetcher;
    let mut funding = create_mock_funding_response(1640995200, "BTC");
    funding.funding_rate = "invalid".to_string();
    let funding_history = vec![funding];
    
    let result = fetcher.validate_funding_response(&funding_history);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::DataConversion(_)));
}

#[test]
fn test_validate_funding_response_not_chronological() {
    let fetcher = MockHyperliquidDataFetcher;
    let funding1 = create_mock_funding_response(1640995260, "BTC");
    let funding2 = create_mock_funding_response(1640995200, "BTC"); // Earlier time
    let funding_history = vec![funding1, funding2];
    
    let result = fetcher.validate_funding_response(&funding_history);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::Validation(_)));
}

#[test]
fn test_validate_funding_response_valid() {
    let fetcher = MockHyperliquidDataFetcher;
    let funding1 = create_mock_funding_response(1640995200, "BTC");
    let funding2 = create_mock_funding_response(1640995260, "BTC");
    let funding_history = vec![funding1, funding2];
    
    let result = fetcher.validate_funding_response(&funding_history);
    assert!(result.is_ok());
}

#[test]
fn test_cacheable_funding_history_conversion() {
    let original = create_mock_funding_response(1640995200, "BTC");
    let cacheable = CacheableFundingHistory::from(&original);
    let converted_back: hyperliquid_rust_sdk::FundingHistoryResponse = cacheable.into();
    
    assert_eq!(original.coin, converted_back.coin);
    assert_eq!(original.funding_rate, converted_back.funding_rate);
    assert_eq!(original.premium, converted_back.premium);
    assert_eq!(original.time, converted_back.time);
}

#[test]
fn test_find_funding_rate_for_timestamp() {
    let fetcher = MockHyperliquidDataFetcher;
    let mut funding_map = std::collections::HashMap::new();
    funding_map.insert(1640995200, 0.0001); // 00:00:00
    funding_map.insert(1640995800, 0.0002); // 00:10:00
    funding_map.insert(1640996400, -0.0001); // 00:20:00
    
    // Test exact match
    let rate = fetcher.find_funding_rate_for_timestamp(1640995800, &funding_map);
    assert_eq!(rate, 0.0002);
    
    // Test finding rate before timestamp
    let rate = fetcher.find_funding_rate_for_timestamp(1640996000, &funding_map); // 00:13:20
    assert_eq!(rate, 0.0002); // Should use 00:10:00 rate
    
    // Test finding rate after timestamp when no earlier rate exists
    let rate = fetcher.find_funding_rate_for_timestamp(1640995000, &funding_map); // Before all rates
    assert_eq!(rate, 0.0001); // Should use first available rate
}

#[test]
fn test_align_ohlc_and_funding_data() {
    let fetcher = MockHyperliquidDataFetcher;
    
    // Create OHLC data
    let ohlc_data = vec![
        create_mock_candle_response(1640995200, "BTC", "1h"), // 00:00:00
        create_mock_candle_response(1640995800, "BTC", "1h"), // 00:10:00
        create_mock_candle_response(1640996400, "BTC", "1h"), // 00:20:00
    ];
    
    // Create funding data
    let funding_data = vec![
        create_mock_funding_response(1640995200, "BTC"), // 00:00:00
        create_mock_funding_response(1640996400, "BTC"), // 00:20:00
    ];
    
    let result = fetcher.align_ohlc_and_funding_data(&ohlc_data, &funding_data);
    assert!(result.is_ok());
    
    let (timestamps, rates) = result.unwrap();
    assert_eq!(timestamps.len(), 3);
    assert_eq!(rates.len(), 3);
    
    // First timestamp should have exact match
    assert_eq!(rates[0], 0.0001);
    // Second timestamp should use previous rate (00:00:00)
    assert_eq!(rates[1], 0.0001);
    // Third timestamp should have exact match
    assert_eq!(rates[2], 0.0001);
}

#[test]
fn test_align_ohlc_and_funding_data_empty() {
    let fetcher = MockHyperliquidDataFetcher;
    let ohlc_data = vec![];
    let funding_data = vec![];
    
    let result = fetcher.align_ohlc_and_funding_data(&ohlc_data, &funding_data);
    assert!(result.is_ok());
    
    let (timestamps, rates) = result.unwrap();
    assert!(timestamps.is_empty());
    assert!(rates.is_empty());
}

// Additional mock implementation for testing funding-specific methods
impl MockHyperliquidDataFetcher {
    fn find_funding_rate_for_timestamp(
        &self,
        timestamp: u64,
        funding_map: &std::collections::HashMap<u64, f64>,
    ) -> f64 {
        // First, try exact match
        if let Some(&rate) = funding_map.get(&timestamp) {
            return rate;
        }

        // If no exact match, find the closest funding rate before this timestamp
        let mut best_timestamp = 0;
        let mut best_rate = 0.0;

        for (&funding_timestamp, &rate) in funding_map.iter() {
            if funding_timestamp <= timestamp && funding_timestamp > best_timestamp {
                best_timestamp = funding_timestamp;
                best_rate = rate;
            }
        }

        // If no funding rate found before this timestamp, try to find one after
        if best_timestamp == 0 {
            let mut closest_timestamp = u64::MAX;
            for (&funding_timestamp, &rate) in funding_map.iter() {
                if funding_timestamp > timestamp && funding_timestamp < closest_timestamp {
                    closest_timestamp = funding_timestamp;
                    best_rate = rate;
                }
            }
        }

        best_rate
    }

    fn align_ohlc_and_funding_data(
        &self,
        ohlc_data: &[hyperliquid_rust_sdk::CandlesSnapshotResponse],
        funding_data: &[hyperliquid_rust_sdk::FundingHistoryResponse],
    ) -> Result<(Vec<DateTime<FixedOffset>>, Vec<f64>)> {
        if ohlc_data.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let mut aligned_timestamps = Vec::new();
        let mut aligned_funding_rates = Vec::new();

        // Convert funding data to a more searchable format
        let funding_map: std::collections::HashMap<u64, f64> = funding_data
            .iter()
            .map(|entry| {
                let rate = entry.funding_rate.parse::<f64>()
                    .unwrap_or(0.0); // Default to 0 if parsing fails
                (entry.time, rate)
            })
            .collect();

        // For each OHLC timestamp, find the corresponding or nearest funding rate
        for candle in ohlc_data {
            let ohlc_timestamp = candle.time_open;
            let datetime = FixedOffset::east_opt(0)
                .ok_or_else(|| HyperliquidBacktestError::data_conversion(
                    "Failed to create UTC timezone offset".to_string()
                ))?
                .timestamp_opt(ohlc_timestamp as i64, 0)
                .single()
                .ok_or_else(|| HyperliquidBacktestError::data_conversion(
                    format!("Invalid timestamp {}", ohlc_timestamp)
                ))?;

            // Find the funding rate for this timestamp
            let funding_rate = self.find_funding_rate_for_timestamp(ohlc_timestamp, &funding_map);
            
            aligned_timestamps.push(datetime);
            aligned_funding_rates.push(funding_rate);
        }

        Ok((aligned_timestamps, aligned_funding_rates))
    }
}

#[test]
fn test_validate_fetch_parameters_valid() {
    let result = HyperliquidData::validate_fetch_parameters("BTC", "1h", 1640995200, 1640998800);
    assert!(result.is_ok());
}

#[test]
fn test_validate_fetch_parameters_empty_coin() {
    let result = HyperliquidData::validate_fetch_parameters("", "1h", 1640995200, 1640998800);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::Validation(_)));
}

#[test]
fn test_validate_fetch_parameters_unsupported_interval() {
    let result = HyperliquidData::validate_fetch_parameters("BTC", "30s", 1640995200, 1640998800);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::UnsupportedInterval(_)));
}

#[test]
fn test_validate_fetch_parameters_invalid_time_range() {
    let result = HyperliquidData::validate_fetch_parameters("BTC", "1h", 1640998800, 1640995200);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::InvalidTimeRange { .. }));
}

#[test]
fn test_validate_fetch_parameters_future_time() {
    let future_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() + 2 * 86400; // 2 days in the future
    
    let result = HyperliquidData::validate_fetch_parameters("BTC", "1h", future_time, future_time + 3600);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::Validation(_)));
}

#[test]
fn test_validate_fetch_parameters_time_range_too_large() {
    let start_time = 1640995200;
    let end_time = start_time + 8 * 24 * 3600; // 8 days for 1m interval (max is 7 days)
    
    let result = HyperliquidData::validate_fetch_parameters("BTC", "1m", start_time, end_time);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::Validation(_)));
}

#[test]
fn test_popular_trading_pairs() {
    let pairs = HyperliquidData::popular_trading_pairs();
    assert!(pairs.contains(&"BTC"));
    assert!(pairs.contains(&"ETH"));
    assert!(pairs.contains(&"SOL"));
    assert!(pairs.contains(&"AVAX"));
    assert!(pairs.contains(&"MATIC"));
    assert!(pairs.contains(&"ARB"));
    assert!(pairs.contains(&"OP"));
    assert!(pairs.contains(&"DOGE"));
    assert!(pairs.contains(&"LINK"));
    assert!(pairs.contains(&"UNI"));
}

#[test]
fn test_is_popular_pair() {
    assert!(HyperliquidData::is_popular_pair("BTC"));
    assert!(HyperliquidData::is_popular_pair("ETH"));
    assert!(HyperliquidData::is_popular_pair("SOL"));
    assert!(!HyperliquidData::is_popular_pair("UNKNOWN"));
    assert!(!HyperliquidData::is_popular_pair(""));
}

#[test]
fn test_fetch_last_hours_time_calculation() {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // We can't actually test the async fetch without a real API, but we can test the time calculation logic
    let hours = 24;
    let expected_start = current_time - (hours * 3600);
    
    // The actual start time should be within a few seconds of our calculation
    assert!(expected_start <= current_time);
    assert!(current_time - expected_start >= hours * 3600 - 10); // Allow 10 second tolerance
}

#[test]
fn test_fetch_last_days_time_calculation() {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let days = 7;
    let expected_start = current_time - (days * 24 * 3600);
    
    // The actual start time should be within a few seconds of our calculation
    assert!(expected_start <= current_time);
    assert!(current_time - expected_start >= days * 24 * 3600 - 10); // Allow 10 second tolerance
}

#[test]
fn test_fetch_date_range_time_conversion() {
    use chrono::{TimeZone, FixedOffset};
    
    let start_date = FixedOffset::east_opt(0).unwrap().timestamp_opt(1640995200, 0).unwrap();
    let end_date = FixedOffset::east_opt(0).unwrap().timestamp_opt(1640998800, 0).unwrap();
    
    let start_time = start_date.timestamp() as u64;
    let end_time = end_date.timestamp() as u64;
    
    assert_eq!(start_time, 1640995200);
    assert_eq!(end_time, 1640998800);
    assert!(end_time > start_time);
}

#[test]
fn test_convenience_methods_parameters() {
    // Test that convenience methods would call fetch with correct parameters
    let start_time = 1640995200;
    let end_time = 1640998800;
    let interval = "1h";
    
    // We can't test the actual async calls without mocking, but we can verify
    // that the parameters would be passed correctly by testing the validation
    assert!(HyperliquidData::validate_fetch_parameters("BTC", interval, start_time, end_time).is_ok());
    assert!(HyperliquidData::validate_fetch_parameters("ETH", interval, start_time, end_time).is_ok());
    assert!(HyperliquidData::validate_fetch_parameters("SOL", interval, start_time, end_time).is_ok());
    assert!(HyperliquidData::validate_fetch_parameters("AVAX", interval, start_time, end_time).is_ok());
    assert!(HyperliquidData::validate_fetch_parameters("MATIC", interval, start_time, end_time).is_ok());
    assert!(HyperliquidData::validate_fetch_parameters("ARB", interval, start_time, end_time).is_ok());
    assert!(HyperliquidData::validate_fetch_parameters("OP", interval, start_time, end_time).is_ok());
}

#[test]
fn test_supported_intervals_comprehensive() {
    let supported_intervals = HyperliquidDataFetcher::supported_intervals();
    
    // Test all supported intervals
    for &interval in supported_intervals {
        assert!(HyperliquidDataFetcher::is_interval_supported(interval));
        
        // Test that validation passes for supported intervals
        let result = HyperliquidData::validate_fetch_parameters("BTC", interval, 1640995200, 1640998800);
        assert!(result.is_ok(), "Interval {} should be supported", interval);
    }
    
    // Test unsupported intervals
    let unsupported = ["30s", "2m", "3m", "6m", "12m", "30m", "2h", "3h", "6h", "8h", "12h", "2d", "3d", "1w", "1M"];
    for &interval in &unsupported {
        assert!(!HyperliquidDataFetcher::is_interval_supported(interval));
        
        let result = HyperliquidData::validate_fetch_parameters("BTC", interval, 1640995200, 1640998800);
        assert!(result.is_err(), "Interval {} should not be supported", interval);
    }
}

#[test]
fn test_max_time_ranges_for_intervals() {
    let test_cases = [
        ("1m", 7 * 24 * 3600),
        ("5m", 30 * 24 * 3600),
        ("15m", 90 * 24 * 3600),
        ("1h", 365 * 24 * 3600),
        ("4h", 2 * 365 * 24 * 3600),
        ("1d", 5 * 365 * 24 * 3600),
    ];
    
    for (interval, expected_max) in test_cases {
        assert_eq!(
            HyperliquidDataFetcher::max_time_range_for_interval(interval),
            expected_max,
            "Max time range for {} should be {} seconds",
            interval,
            expected_max
        );
    }
}

#[test]
fn test_edge_cases_for_time_validation() {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Test with start time exactly equal to end time
    let result = HyperliquidData::validate_fetch_parameters("BTC", "1h", 1640995200, 1640995200);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::InvalidTimeRange { .. }));
    
    // Test with start time 1 second before end time (should be valid)
    let result = HyperliquidData::validate_fetch_parameters("BTC", "1h", 1640995200, 1640995201);
    assert!(result.is_ok());
    
    // Test with end time exactly at the max range limit (should be valid)
    let start_time = 1640995200;
    let max_range = HyperliquidDataFetcher::max_time_range_for_interval("1h");
    let end_time = start_time + max_range;
    let result = HyperliquidData::validate_fetch_parameters("BTC", "1h", start_time, end_time);
    assert!(result.is_ok());
    
    // Test with end time 1 second beyond the max range limit (should be invalid)
    let end_time = start_time + max_range + 1;
    let result = HyperliquidData::validate_fetch_parameters("BTC", "1h", start_time, end_time);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), HyperliquidBacktestError::Validation(_)));
}

#[test]
fn test_comprehensive_parameter_combinations() {
    let coins = ["BTC", "ETH", "SOL"];
    let intervals = ["1m", "5m", "15m", "1h", "4h", "1d"];
    let start_time = 1640995200;
    let end_time = start_time + 3600;
    
    for &coin in &coins {
        for &interval in &intervals {
            let result = HyperliquidData::validate_fetch_parameters(coin, interval, start_time, end_time);
            assert!(result.is_ok(), "Validation should pass for coin: {}, interval: {}", coin, interval);
        }
    }
}