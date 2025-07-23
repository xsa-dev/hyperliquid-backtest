//! Mock data generators for testing

use crate::data::HyperliquidData;
use crate::backtest::{FundingPayment, HyperliquidCommission};
use chrono::{DateTime, FixedOffset, TimeZone, Duration};
use hyperliquid_rust_sdk::{CandlesSnapshotResponse, FundingHistoryResponse};
use std::collections::HashMap;

/// Generate mock OHLC data with funding rates
pub fn generate_mock_data(
    symbol: &str,
    hours: usize,
    with_funding: bool,
    with_gaps: bool,
) -> HyperliquidData {
    let mut datetime = Vec::new();
    let mut open = Vec::new();
    let mut high = Vec::new();
    let mut low = Vec::new();
    let mut close = Vec::new();
    let mut volume = Vec::new();
    let mut funding_rates = Vec::new();
    
    // Create hourly data
    let base_timestamp = 1640995200; // 2022-01-01 00:00:00 UTC
    
    for i in 0..hours {
        // Skip some hours if with_gaps is true
        if with_gaps && i % 10 == 0 {
            continue;
        }
        
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(base_timestamp + i as i64 * 3600, 0).unwrap();
        
        datetime.push(timestamp);
        
        // Create a price pattern with some trend and volatility
        let trend = (i as f64) * 0.01;
        let cycle = ((i as f64) * 0.1).sin() * 5.0;
        let price = 100.0 + trend + cycle;
        
        open.push(price - 0.5);
        high.push(price + 1.0);
        low.push(price - 1.0);
        close.push(price);
        volume.push(1000.0 + (i as f64 % 24.0) * 100.0); // Higher volume during certain hours
        
        if with_funding {
            // Add funding rates every 8 hours (0:00, 8:00, 16:00)
            if timestamp.hour() % 8 == 0 {
                // Create funding rate pattern
                let funding_cycle = ((i as f64) * 0.05).sin() * 0.0002;
                funding_rates.push(funding_cycle);
            } else {
                funding_rates.push(f64::NAN);
            }
        } else {
            funding_rates.push(f64::NAN);
        }
    }
    
    HyperliquidData {
        symbol: symbol.to_string(),
        datetime,
        open,
        high,
        low,
        close,
        volume,
        funding_rates,
    }
}

/// Generate mock Hyperliquid API candles response
pub fn generate_mock_candles_response(
    hours: usize,
    with_gaps: bool,
) -> Vec<CandlesSnapshotResponse> {
    let mut candles = Vec::new();
    let base_timestamp = 1640995200; // 2022-01-01 00:00:00 UTC
    
    for i in 0..hours {
        // Skip some hours if with_gaps is true
        if with_gaps && i % 10 == 0 {
            continue;
        }
        
        let time_open = base_timestamp + i as u64 * 3600;
        let time_close = time_open + 3600;
        
        // Create a price pattern with some trend and volatility
        let trend = (i as f64) * 0.01;
        let cycle = ((i as f64) * 0.1).sin() * 5.0;
        let price = 100.0 + trend + cycle;
        
        candles.push(CandlesSnapshotResponse {
            time_open,
            time_close,
            open: format!("{:.2}", price - 0.5),
            high: format!("{:.2}", price + 1.0),
            low: format!("{:.2}", price - 1.0),
            close: format!("{:.2}", price),
            vlm: format!("{:.2}", 1000.0 + (i as f64 % 24.0) * 100.0),
        });
    }
    
    candles
}

/// Generate mock funding history response
pub fn generate_mock_funding_history(
    coin: &str,
    hours: usize,
    with_gaps: bool,
) -> Vec<FundingHistoryResponse> {
    let mut funding_history = Vec::new();
    let base_timestamp = 1640995200; // 2022-01-01 00:00:00 UTC
    
    for i in 0..hours {
        // Only include funding rates every 8 hours
        if i % 8 != 0 {
            continue;
        }
        
        // Skip some funding periods if with_gaps is true
        if with_gaps && i % 24 == 0 {
            continue;
        }
        
        let timestamp = base_timestamp + i as u64 * 3600;
        
        // Create funding rate pattern
        let funding_cycle = ((i as f64) * 0.05).sin() * 0.0002;
        
        funding_history.push(FundingHistoryResponse {
            coin: coin.to_string(),
            funding_rate: format!("{:.8}", funding_cycle),
            premium: format!("{:.8}", funding_cycle * 3.0), // Just a mock value
            time: timestamp,
        });
    }
    
    funding_history
}

/// Generate mock funding payments
pub fn generate_mock_funding_payments(
    hours: usize,
    position_size: f64,
) -> Vec<FundingPayment> {
    let mut payments = Vec::new();
    let base_timestamp = 1640995200; // 2022-01-01 00:00:00 UTC
    
    for i in 0..hours {
        // Only include funding payments every 8 hours
        if i % 8 != 0 {
            continue;
        }
        
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(base_timestamp + i as i64 * 3600, 0).unwrap();
        
        // Create funding rate pattern
        let funding_rate = ((i as f64) * 0.05).sin() * 0.0002;
        let price = 100.0 + (i as f64) * 0.01;
        
        // Calculate payment amount
        let payment_amount = -position_size * funding_rate * price;
        
        payments.push(FundingPayment {
            timestamp,
            funding_rate,
            position_size,
            price,
            payment_amount,
        });
    }
    
    payments
}

/// Generate a sequence of positions for testing
pub fn generate_position_sequence(
    length: usize,
    pattern: &str,
) -> Vec<f64> {
    match pattern {
        "constant_long" => vec![1.0; length],
        "constant_short" => vec![-1.0; length],
        "alternating" => (0..length).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect(),
        "increasing" => (0..length).map(|i| (i as f64 % 5.0) * 0.2).collect(),
        "decreasing" => (0..length).map(|i| 1.0 - (i as f64 % 5.0) * 0.2).collect(),
        "zero" => vec![0.0; length],
        _ => vec![1.0; length], // Default to constant long
    }
}

/// Generate a datetime sequence
pub fn generate_datetime_sequence(
    start_timestamp: i64,
    count: usize,
    interval_seconds: i64,
) -> Vec<DateTime<FixedOffset>> {
    let tz = FixedOffset::east_opt(0).unwrap();
    (0..count)
        .map(|i| tz.timestamp_opt(start_timestamp + i as i64 * interval_seconds, 0).unwrap())
        .collect()
}

/// Generate invalid data for testing error cases
pub fn generate_invalid_data() -> HashMap<&'static str, HyperliquidData> {
    let mut invalid_data = HashMap::new();
    
    // Empty data
    invalid_data.insert("empty", HyperliquidData {
        symbol: "BTC".to_string(),
        datetime: Vec::new(),
        open: Vec::new(),
        high: Vec::new(),
        low: Vec::new(),
        close: Vec::new(),
        volume: Vec::new(),
        funding_rates: Vec::new(),
    });
    
    // Mismatched array lengths
    let mut mismatched = generate_mock_data("BTC", 24, false, false);
    mismatched.open.pop(); // Remove last element to create length mismatch
    invalid_data.insert("mismatched_lengths", mismatched);
    
    // Invalid high/low (high < low)
    let mut invalid_high_low = generate_mock_data("BTC", 24, false, false);
    // Swap high and low for the first candle
    let temp = invalid_high_low.high[0];
    invalid_high_low.high[0] = invalid_high_low.low[0];
    invalid_high_low.low[0] = temp;
    invalid_data.insert("invalid_high_low", invalid_high_low);
    
    // Non-chronological timestamps
    let mut non_chronological = generate_mock_data("BTC", 24, false, false);
    // Swap two timestamps to break chronological order
    let temp = non_chronological.datetime[5];
    non_chronological.datetime[5] = non_chronological.datetime[10];
    non_chronological.datetime[10] = temp;
    invalid_data.insert("non_chronological", non_chronological);
    
    invalid_data
}

/// Generate mock commission configurations for testing
pub fn generate_commission_configs() -> Vec<HyperliquidCommission> {
    vec![
        HyperliquidCommission::default(),
        HyperliquidCommission::new(0.0001, 0.0003, true),
        HyperliquidCommission::new(0.0, 0.0, false),
        HyperliquidCommission::new(0.001, 0.002, true),
    ]
}