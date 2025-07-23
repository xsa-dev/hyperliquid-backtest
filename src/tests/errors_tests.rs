//! Tests for the error handling module

use crate::errors::*;
use chrono::{DateTime, FixedOffset, TimeZone};
use std::io;
use std::error::Error;

#[test]
fn test_error_display_formatting() {
    let error = HyperliquidBacktestError::HyperliquidApi("API connection failed".to_string());
    assert_eq!(error.to_string(), "Hyperliquid API error: API connection failed");

    let error = HyperliquidBacktestError::DataConversion("Invalid OHLC format".to_string());
    assert_eq!(error.to_string(), "Data conversion error: Invalid OHLC format");

    let error = HyperliquidBacktestError::InvalidTimeRange { start: 1000, end: 500 };
    assert_eq!(error.to_string(), "Invalid time range: start 1000 >= end 500");

    let error = HyperliquidBacktestError::UnsupportedInterval("30s".to_string());
    assert_eq!(error.to_string(), "Unsupported interval: 30s");
}

#[test]
fn test_missing_funding_data_error() {
    let dt = TimeZone::timestamp_opt(&FixedOffset::east_opt(0).unwrap(), 1640995200, 0).unwrap();
    let error = HyperliquidBacktestError::MissingFundingData(dt);
    assert!(error.to_string().contains("Missing funding data for timestamp"));
}

#[test]
fn test_error_conversions_from_std_errors() {
    // Test ParseFloatError conversion
    let parse_error: std::result::Result<f64, std::num::ParseFloatError> = "not_a_number".parse();
    let converted_error: HyperliquidBacktestError = parse_error.unwrap_err().into();
    match converted_error {
        HyperliquidBacktestError::NumberParsing(_) => {},
        _ => panic!("Expected NumberParsing error"),
    }

    // Test ParseIntError conversion
    let parse_error: std::result::Result<i32, std::num::ParseIntError> = "not_a_number".parse();
    let converted_error: HyperliquidBacktestError = parse_error.unwrap_err().into();
    match converted_error {
        HyperliquidBacktestError::NumberParsing(_) => {},
        _ => panic!("Expected NumberParsing error"),
    }
}

#[test]
fn test_io_error_conversion() {
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let converted_error: HyperliquidBacktestError = io_error.into();
    match converted_error {
        HyperliquidBacktestError::Io(_) => {},
        _ => panic!("Expected Io error"),
    }
}

#[test]
fn test_json_error_conversion() {
    let json_str = r#"{"invalid": json}"#;
    let json_error = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
    let converted_error: HyperliquidBacktestError = json_error.into();
    match converted_error {
        HyperliquidBacktestError::JsonParsing(_) => {},
        _ => panic!("Expected JsonParsing error"),
    }
}

#[test]
fn test_chrono_parse_error_conversion() {
    let parse_result = DateTime::parse_from_rfc3339("invalid-date");
    let parse_error = parse_result.unwrap_err();
    let converted_error: HyperliquidBacktestError = parse_error.into();
    match converted_error {
        HyperliquidBacktestError::DateTimeParsing(_) => {},
        _ => panic!("Expected DateTimeParsing error"),
    }
}

#[test]
fn test_error_debug_formatting() {
    let error = HyperliquidBacktestError::Validation("Invalid parameter".to_string());
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("Validation"));
    assert!(debug_str.contains("Invalid parameter"));
}

#[test]
fn test_result_type_alias() {
    fn test_function() -> Result<i32> {
        Ok(42)
    }

    fn test_error_function() -> Result<i32> {
        Err(HyperliquidBacktestError::Configuration("Test error".to_string()))
    }

    assert_eq!(test_function().unwrap(), 42);
    assert!(test_error_function().is_err());
}

#[test]
fn test_error_chain_with_source() {
    let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
    let converted_error: HyperliquidBacktestError = io_error.into();
    
    // Test that the error chain is preserved
    assert!(converted_error.source().is_some());
}

#[test]
fn test_all_error_variants_display() {
    let test_cases = vec![
        HyperliquidBacktestError::HyperliquidApi("test".to_string()),
        HyperliquidBacktestError::DataConversion("test".to_string()),
        HyperliquidBacktestError::InvalidTimeRange { start: 100, end: 50 },
        HyperliquidBacktestError::UnsupportedInterval("test".to_string()),
        HyperliquidBacktestError::MissingFundingData(
            TimeZone::timestamp_opt(&FixedOffset::east_opt(0).unwrap(), 0, 0).unwrap()
        ),
        HyperliquidBacktestError::Backtesting("test".to_string()),
        HyperliquidBacktestError::Network("test".to_string()),
        HyperliquidBacktestError::Http("test".to_string()),
        HyperliquidBacktestError::NumberParsing("test".to_string()),
        HyperliquidBacktestError::Configuration("test".to_string()),
        HyperliquidBacktestError::Validation("test".to_string()),
        HyperliquidBacktestError::RateLimit("test".to_string()),
        HyperliquidBacktestError::Authentication("test".to_string()),
        HyperliquidBacktestError::DataIntegrity("test".to_string()),
    ];

    for error in test_cases {
        // Ensure all error variants can be displayed without panicking
        let _display = error.to_string();
        let _debug = format!("{:?}", error);
    }
}

#[test]
fn test_error_categorization() {
    // Test that errors can be categorized appropriately
    let network_error = HyperliquidBacktestError::Network("Connection failed".to_string());
    let data_error = HyperliquidBacktestError::DataConversion("Invalid format".to_string());
    let validation_error = HyperliquidBacktestError::Validation("Invalid input".to_string());

    // These should be different error types
    assert!(matches!(network_error, HyperliquidBacktestError::Network(_)));
    assert!(matches!(data_error, HyperliquidBacktestError::DataConversion(_)));
    assert!(matches!(validation_error, HyperliquidBacktestError::Validation(_)));
}

#[tokio::test]
async fn test_tokio_join_error_conversion() {
    let handle = tokio::spawn(async {
        panic!("Test panic");
    });

    let join_result = handle.await;
    assert!(join_result.is_err());
    
    let join_error = join_result.unwrap_err();
    let converted_error: HyperliquidBacktestError = join_error.into();
    
    match converted_error {
        HyperliquidBacktestError::Backtesting(msg) => {
            assert!(msg.contains("Task join error"));
        },
        _ => panic!("Expected Backtesting error"),
    }
}

#[test]
fn test_error_equality_and_matching() {
    // Test that we can match on error variants
    let error = HyperliquidBacktestError::RateLimit("Too many requests".to_string());
    
    match error {
        HyperliquidBacktestError::RateLimit(msg) => {
            assert_eq!(msg, "Too many requests");
        },
        _ => panic!("Expected RateLimit error"),
    }
}

#[test]
fn test_error_helper_methods() {
    // Test helper method constructors
    let api_error = HyperliquidBacktestError::hyperliquid_api("API failed");
    assert!(matches!(api_error, HyperliquidBacktestError::HyperliquidApi(_)));
    assert_eq!(api_error.to_string(), "Hyperliquid API error: API failed");

    let conversion_error = HyperliquidBacktestError::data_conversion("Invalid format");
    assert!(matches!(conversion_error, HyperliquidBacktestError::DataConversion(_)));
    assert_eq!(conversion_error.to_string(), "Data conversion error: Invalid format");

    let time_range_error = HyperliquidBacktestError::invalid_time_range(1000, 500);
    assert!(matches!(time_range_error, HyperliquidBacktestError::InvalidTimeRange { .. }));
    assert_eq!(time_range_error.to_string(), "Invalid time range: start 1000 >= end 500");

    let interval_error = HyperliquidBacktestError::unsupported_interval("30s");
    assert!(matches!(interval_error, HyperliquidBacktestError::UnsupportedInterval(_)));
    assert_eq!(interval_error.to_string(), "Unsupported interval: 30s");

    let dt = TimeZone::timestamp_opt(&FixedOffset::east_opt(0).unwrap(), 1640995200, 0).unwrap();
    let funding_error = HyperliquidBacktestError::missing_funding_data(dt);
    assert!(matches!(funding_error, HyperliquidBacktestError::MissingFundingData(_)));

    let validation_error = HyperliquidBacktestError::validation("Invalid input");
    assert!(matches!(validation_error, HyperliquidBacktestError::Validation(_)));
    assert_eq!(validation_error.to_string(), "Validation error: Invalid input");

    let network_error = HyperliquidBacktestError::network("Connection failed");
    assert!(matches!(network_error, HyperliquidBacktestError::Network(_)));
    assert_eq!(network_error.to_string(), "Network error: Connection failed");

    let rate_limit_error = HyperliquidBacktestError::rate_limit("Too many requests");
    assert!(matches!(rate_limit_error, HyperliquidBacktestError::RateLimit(_)));
    assert_eq!(rate_limit_error.to_string(), "Rate limit exceeded: Too many requests");
}

#[test]
fn test_error_helper_methods_with_string_types() {
    // Test that helper methods work with different string types
    let owned_string = String::from("test message");
    let error1 = HyperliquidBacktestError::validation(&owned_string);
    let error2 = HyperliquidBacktestError::validation(owned_string.clone());
    let error3 = HyperliquidBacktestError::validation("test message");

    assert_eq!(error1.to_string(), error2.to_string());
    assert_eq!(error2.to_string(), error3.to_string());
}