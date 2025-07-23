//! Tests for funding payment calculations

use crate::backtest::{
    HyperliquidBacktest, HyperliquidCommission, FundingPayment,
    EnhancedMetrics, OrderTypeStrategy
};
use crate::data::HyperliquidData;
use crate::errors::Result;
use chrono::{DateTime, FixedOffset, TimeZone, Timelike};

// Helper function to create test data
fn create_test_data() -> HyperliquidData {
    let mut datetime = Vec::new();
    let mut open = Vec::new();
    let mut high = Vec::new();
    let mut low = Vec::new();
    let mut close = Vec::new();
    let mut volume = Vec::new();
    let mut funding_rates = Vec::new();
    
    // Create 3 days of hourly data (72 hours)
    let base_timestamp = 1640995200; // 2022-01-01 00:00:00 UTC
    
    for i in 0..72 {
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(base_timestamp + i * 3600, 0).unwrap();
        
        datetime.push(timestamp);
        open.push(100.0 + (i as f64 * 0.1));
        high.push(101.0 + (i as f64 * 0.1));
        low.push(99.0 + (i as f64 * 0.1));
        close.push(100.5 + (i as f64 * 0.1));
        volume.push(1000.0 + (i as f64 * 10.0));
        
        // Add funding rates every 8 hours (0:00, 8:00, 16:00)
        if timestamp.hour() % 8 == 0 {
            // Alternate between positive and negative funding
            let funding_rate = if (i / 8) % 2 == 0 { 0.0001 } else { -0.0001 };
            funding_rates.push(funding_rate);
        } else {
            funding_rates.push(f64::NAN);
        }
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
fn test_is_funding_time() {
    let data = create_test_data();
    let backtest = HyperliquidBacktest::new(
        data.clone(),
        "test_strategy".to_string(),
        10000.0,
        HyperliquidCommission::default(),
    );
    
    // Test funding times (every 8 hours: 00:00, 08:00, 16:00)
    for i in 0..data.datetime.len() {
        let timestamp = data.datetime[i];
        let expected = timestamp.hour() % 8 == 0 && timestamp.minute() == 0 && timestamp.second() == 0;
        assert_eq!(backtest.is_funding_time(timestamp), expected);
    }
}

#[test]
fn test_get_funding_rate_for_timestamp() {
    let data = create_test_data();
    let backtest = HyperliquidBacktest::new(
        data.clone(),
        "test_strategy".to_string(),
        10000.0,
        HyperliquidCommission::default(),
    );
    
    // Test funding rate lookup
    for i in 0..data.datetime.len() {
        let timestamp = data.datetime[i];
        let funding_rate = backtest.get_funding_rate_for_timestamp(timestamp);
        
        if timestamp.hour() % 8 == 0 {
            // Should have a valid funding rate
            assert!(funding_rate.is_some());
            let expected_rate = if (i / 8) % 2 == 0 { 0.0001 } else { -0.0001 };
            assert_eq!(funding_rate.unwrap(), expected_rate);
        } else {
            // No funding rate at non-funding times
            assert!(funding_rate.is_none());
        }
    }
}

#[test]
fn test_calculate_funding_payment() {
    let data = create_test_data();
    let backtest = HyperliquidBacktest::new(
        data.clone(),
        "test_strategy".to_string(),
        10000.0,
        HyperliquidCommission::default(),
    );
    
    // Test with long position and positive funding rate
    let payment1 = backtest.calculate_funding_payment(1.0, 0.0001, 100.0);
    // Long position pays when funding rate is positive: -1.0 * 0.0001 * 100.0 = -0.01
    assert_eq!(payment1, -0.01);
    
    // Test with long position and negative funding rate
    let payment2 = backtest.calculate_funding_payment(1.0, -0.0001, 100.0);
    // Long position receives when funding rate is negative: -1.0 * -0.0001 * 100.0 = 0.01
    assert_eq!(payment2, 0.01);
    
    // Test with short position and positive funding rate
    let payment3 = backtest.calculate_funding_payment(-1.0, 0.0001, 100.0);
    // Short position receives when funding rate is positive: -(-1.0) * 0.0001 * 100.0 = 0.01
    assert_eq!(payment3, 0.01);
    
    // Test with short position and negative funding rate
    let payment4 = backtest.calculate_funding_payment(-1.0, -0.0001, 100.0);
    // Short position pays when funding rate is negative: -(-1.0) * -0.0001 * 100.0 = -0.01
    assert_eq!(payment4, -0.01);
    
    // Test with zero position
    let payment5 = backtest.calculate_funding_payment(0.0, 0.0001, 100.0);
    // No position means no funding payment
    assert_eq!(payment5, 0.0);
}

#[test]
fn test_calculate_with_funding() -> Result<()> {
    let data = create_test_data();
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        "test_strategy".to_string(),
        10000.0,
        HyperliquidCommission::default(),
    );
    
    // Initialize the base backtest
    backtest.initialize_base_backtest()?;
    
    // Calculate with funding
    backtest.calculate_with_funding()?;
    
    // Verify funding payments were calculated
    assert!(!backtest.funding_payments.is_empty());
    
    // There should be 9 funding payments (3 days * 3 payments per day)
    assert_eq!(backtest.funding_payments.len(), 9);
    
    // Verify funding PnL tracking
    assert_eq!(backtest.funding_pnl.len(), data.len());
    
    // Verify enhanced metrics were updated
    assert!(backtest.enhanced_metrics.funding_payments_received >= 0);
    assert!(backtest.enhanced_metrics.funding_payments_paid >= 0);
    
    Ok(())
}

#[test]
fn test_calculate_with_funding_and_positions() -> Result<()> {
    let data = create_test_data();
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        "test_strategy".to_string(),
        10000.0,
        HyperliquidCommission::default(),
    );
    
    // Initialize the base backtest
    backtest.initialize_base_backtest()?;
    
    // Create position array (alternating between long and short)
    let positions: Vec<f64> = (0..data.len())
        .map(|i| if i % 16 < 8 { 1.0 } else { -1.0 })
        .collect();
    
    // Calculate with funding and positions
    backtest.calculate_with_funding_and_positions(&positions)?;
    
    // Verify funding payments were calculated
    assert!(!backtest.funding_payments.is_empty());
    
    // There should be 9 funding payments (3 days * 3 payments per day)
    assert_eq!(backtest.funding_payments.len(), 9);
    
    // Verify position sizes in funding payments
    for (i, payment) in backtest.funding_payments.iter().enumerate() {
        let expected_position = if (i / 3) % 2 == 0 { 1.0 } else { -1.0 };
        assert_eq!(payment.position_size, expected_position);
    }
    
    // Verify funding PnL tracking
    assert_eq!(backtest.funding_pnl.len(), data.len());
    
    // Verify enhanced metrics were updated
    assert!(backtest.enhanced_metrics.funding_payments_received > 0);
    assert!(backtest.enhanced_metrics.funding_payments_paid > 0);
    
    Ok(())
}

#[test]
fn test_calculate_with_funding_invalid_state() {
    let data = create_test_data();
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        "test_strategy".to_string(),
        10000.0,
        HyperliquidCommission::default(),
    );
    
    // Try to calculate funding without initializing base backtest
    let result = backtest.calculate_with_funding();
    
    // Should fail with validation error
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Base backtest must be initialized"));
    }
}

#[test]
fn test_calculate_with_funding_and_positions_invalid_length() -> Result<()> {
    let data = create_test_data();
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        "test_strategy".to_string(),
        10000.0,
        HyperliquidCommission::default(),
    );
    
    // Initialize the base backtest
    backtest.initialize_base_backtest()?;
    
    // Create position array with wrong length
    let positions = vec![1.0; data.len() - 10];
    
    // Try to calculate with invalid positions array
    let result = backtest.calculate_with_funding_and_positions(&positions);
    
    // Should fail with validation error
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Positions array length must match data length"));
    }
    
    Ok(())
}

#[test]
fn test_update_enhanced_metrics() -> Result<()> {
    let data = create_test_data();
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        "test_strategy".to_string(),
        10000.0,
        HyperliquidCommission::default(),
    );
    
    // Initialize the base backtest
    backtest.initialize_base_backtest()?;
    
    // Create position array with alternating positions
    let positions: Vec<f64> = (0..data.len())
        .map(|i| if i % 16 < 8 { 1.0 } else { -1.0 })
        .collect();
    
    // Calculate with funding and positions
    backtest.calculate_with_funding_and_positions(&positions)?;
    
    // Verify enhanced metrics
    let metrics = &backtest.enhanced_metrics;
    
    // Check that metrics were calculated
    assert!(metrics.funding_only_return != 0.0);
    assert!(metrics.funding_payments_received + metrics.funding_payments_paid > 0);
    assert!(metrics.average_funding_rate != 0.0);
    
    // Verify total return calculation
    assert_eq!(
        metrics.total_return_with_funding,
        metrics.trading_only_return + metrics.funding_only_return
    );
    
    Ok(())
}

#[test]
fn test_funding_summary() -> Result<()> {
    let data = create_test_data();
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        "test_strategy".to_string(),
        10000.0,
        HyperliquidCommission::default(),
    );
    
    // Initialize the base backtest
    backtest.initialize_base_backtest()?;
    
    // Create position array with constant long position
    let positions = vec![1.0; data.len()];
    
    // Calculate with funding and positions
    backtest.calculate_with_funding_and_positions(&positions)?;
    
    // Get funding summary
    let summary = backtest.funding_summary();
    
    // Verify summary calculations
    assert_eq!(summary.total_funding_paid, backtest.total_funding_paid);
    assert_eq!(summary.total_funding_received, backtest.total_funding_received);
    assert_eq!(summary.net_funding, backtest.total_funding_received - backtest.total_funding_paid);
    assert_eq!(summary.funding_payment_count, backtest.funding_payments.len());
    
    // Check average funding payment calculation
    if !backtest.funding_payments.is_empty() {
        let total_payments: f64 = backtest.funding_payments.iter()
            .map(|p| p.payment_amount)
            .sum();
        let expected_avg = total_payments / backtest.funding_payments.len() as f64;
        assert_eq!(summary.average_funding_payment, expected_avg);
    }
    
    Ok(())
}