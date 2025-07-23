//! Tests for the HyperliquidData price lookup functionality

use crate::data::HyperliquidData;
use crate::errors::Result;
use chrono::{DateTime, FixedOffset, TimeZone};

// Helper function to create test data
fn create_test_data() -> HyperliquidData {
    let mut datetime = Vec::new();
    let mut open = Vec::new();
    let mut high = Vec::new();
    let mut low = Vec::new();
    let mut close = Vec::new();
    let mut volume = Vec::new();
    let mut funding_rates = Vec::new();
    
    // Create hourly data for 24 hours
    let base_timestamp = 1640995200; // 2022-01-01 00:00:00 UTC
    
    for i in 0..24 {
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(base_timestamp + i * 3600, 0).unwrap();
        
        datetime.push(timestamp);
        open.push(100.0 + (i as f64));
        high.push(105.0 + (i as f64));
        low.push(95.0 + (i as f64));
        close.push(102.0 + (i as f64));
        volume.push(1000.0 + (i as f64 * 100.0));
        funding_rates.push(0.0001);
    }
    
    HyperliquidData {
        symbol: "BTC".to_string(),
        datetime,
        open,
        high,
        low,
        close,
        volume,
        funding_rates,
    }
}

#[test]
fn test_get_price_at_or_near_exact_match() {
    let data = create_test_data();
    
    // Test exact timestamp match
    let timestamp = FixedOffset::east_opt(0).unwrap()
        .timestamp_opt(1640995200, 0).unwrap(); // First timestamp
    
    let price = data.get_price_at_or_near(timestamp);
    assert!(price.is_some());
    assert_eq!(price.unwrap(), 102.0); // First close price
}

#[test]
fn test_get_price_at_or_near_close_match() {
    let data = create_test_data();
    
    // Test timestamp that's close but not exact (30 minutes after first entry)
    let timestamp = FixedOffset::east_opt(0).unwrap()
        .timestamp_opt(1640995200 + 1800, 0).unwrap();
    
    let price = data.get_price_at_or_near(timestamp);
    assert!(price.is_some());
    assert_eq!(price.unwrap(), 102.0); // Should return first close price
}

#[test]
fn test_get_price_at_or_near_between_entries() {
    let data = create_test_data();
    
    // Test timestamp between two entries (30 minutes after second entry)
    let timestamp = FixedOffset::east_opt(0).unwrap()
        .timestamp_opt(1640995200 + 3600 + 1800, 0).unwrap();
    
    let price = data.get_price_at_or_near(timestamp);
    assert!(price.is_some());
    assert_eq!(price.unwrap(), 103.0); // Should return second close price
}

#[test]
fn test_get_price_at_or_near_outside_range_but_within_window() {
    let data = create_test_data();
    
    // Test timestamp just before first entry (30 minutes before)
    let timestamp = FixedOffset::east_opt(0).unwrap()
        .timestamp_opt(1640995200 - 1800, 0).unwrap();
    
    let price = data.get_price_at_or_near(timestamp);
    assert!(price.is_some());
    assert_eq!(price.unwrap(), 102.0); // Should return first close price
    
    // Test timestamp just after last entry (30 minutes after)
    let last_timestamp = 1640995200 + 23 * 3600;
    let timestamp = FixedOffset::east_opt(0).unwrap()
        .timestamp_opt(last_timestamp + 1800, 0).unwrap();
    
    let price = data.get_price_at_or_near(timestamp);
    assert!(price.is_some());
    assert_eq!(price.unwrap(), 125.0); // Should return last close price
}

#[test]
fn test_get_price_at_or_near_far_outside_range() {
    let data = create_test_data();
    
    // Test timestamp far before first entry (2 days before)
    let timestamp = FixedOffset::east_opt(0).unwrap()
        .timestamp_opt(1640995200 - 2 * 24 * 3600, 0).unwrap();
    
    let price = data.get_price_at_or_near(timestamp);
    assert!(price.is_none()); // Should return None as it's outside the 24-hour window
    
    // Test timestamp far after last entry (2 days after)
    let last_timestamp = 1640995200 + 23 * 3600;
    let timestamp = FixedOffset::east_opt(0).unwrap()
        .timestamp_opt(last_timestamp + 2 * 24 * 3600, 0).unwrap();
    
    let price = data.get_price_at_or_near(timestamp);
    assert!(price.is_none()); // Should return None as it's outside the 24-hour window
}

#[test]
fn test_get_price_at_or_near_empty_data() {
    let data = HyperliquidData {
        symbol: "BTC".to_string(),
        datetime: Vec::new(),
        open: Vec::new(),
        high: Vec::new(),
        low: Vec::new(),
        close: Vec::new(),
        volume: Vec::new(),
        funding_rates: Vec::new(),
    };
    
    let timestamp = FixedOffset::east_opt(0).unwrap()
        .timestamp_opt(1640995200, 0).unwrap();
    
    let price = data.get_price_at_or_near(timestamp);
    assert!(price.is_none()); // Should return None for empty data
}

#[test]
fn test_get_price_at_or_near_edge_cases() {
    // Create data with just one entry
    let mut data = HyperliquidData {
        symbol: "BTC".to_string(),
        datetime: Vec::new(),
        open: Vec::new(),
        high: Vec::new(),
        low: Vec::new(),
        close: Vec::new(),
        volume: Vec::new(),
        funding_rates: Vec::new(),
    };
    
    let timestamp = FixedOffset::east_opt(0).unwrap()
        .timestamp_opt(1640995200, 0).unwrap();
    
    data.datetime.push(timestamp);
    data.open.push(100.0);
    data.high.push(105.0);
    data.low.push(95.0);
    data.close.push(102.0);
    data.volume.push(1000.0);
    data.funding_rates.push(0.0001);
    
    // Test exact match with single entry
    let price = data.get_price_at_or_near(timestamp);
    assert!(price.is_some());
    assert_eq!(price.unwrap(), 102.0);
    
    // Test near match with single entry
    let near_timestamp = FixedOffset::east_opt(0).unwrap()
        .timestamp_opt(1640995200 + 1800, 0).unwrap();
    
    let price = data.get_price_at_or_near(near_timestamp);
    assert!(price.is_some());
    assert_eq!(price.unwrap(), 102.0);
}

#[test]
fn test_get_price_at_or_near_performance() {
    // Create a large dataset to test performance
    let mut data = HyperliquidData {
        symbol: "BTC".to_string(),
        datetime: Vec::new(),
        open: Vec::new(),
        high: Vec::new(),
        low: Vec::new(),
        close: Vec::new(),
        volume: Vec::new(),
        funding_rates: Vec::new(),
    };
    
    // Create hourly data for 365 days
    let base_timestamp = 1640995200; // 2022-01-01 00:00:00 UTC
    
    for i in 0..365*24 {
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(base_timestamp + i * 3600, 0).unwrap();
        
        data.datetime.push(timestamp);
        data.open.push(100.0 + (i as f64 * 0.01));
        data.high.push(105.0 + (i as f64 * 0.01));
        data.low.push(95.0 + (i as f64 * 0.01));
        data.close.push(102.0 + (i as f64 * 0.01));
        data.volume.push(1000.0);
        data.funding_rates.push(0.0001);
    }
    
    // Test 100 random lookups
    use std::time::Instant;
    let start = Instant::now();
    
    for i in 0..100 {
        let random_offset = (i * 37) % (365 * 24); // Pseudo-random offset
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(base_timestamp + random_offset * 3600 + 1800, 0).unwrap();
        
        let price = data.get_price_at_or_near(timestamp);
        assert!(price.is_some());
    }
    
    let duration = start.elapsed();
    println!("100 price lookups in large dataset took: {:?}", duration);
    
    // This is not a strict test, but should complete in reasonable time
    // If this becomes too slow, it would indicate a performance regression
    assert!(duration.as_secs() < 1, "Price lookup performance test took too long");
}