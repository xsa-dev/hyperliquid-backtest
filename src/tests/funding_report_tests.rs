//! Tests for the funding report module

use crate::funding_report::*;
use crate::backtest::FundingPayment;
use crate::data::HyperliquidData;
use crate::errors::Result;
use crate::tests::mock_data::{
    generate_mock_data, generate_mock_funding_payments,
    generate_position_sequence
};
use chrono::{DateTime, FixedOffset, TimeZone};
use std::collections::HashMap;

#[test]
fn test_funding_report_creation() -> Result<()> {
    // Create test data
    let data = generate_mock_data("BTC", 72, true, false);
    let positions = generate_position_sequence(data.len(), "alternating");
    let payments = generate_mock_funding_payments(72, 1.0);
    let net_funding_pnl = payments.iter().map(|p| p.payment_amount).sum();
    
    // Create funding report
    let report = FundingReport::new(
        "BTC",
        &data,
        &positions,
        payments.clone(),
        net_funding_pnl,
    )?;
    
    // Verify report fields
    assert_eq!(report.symbol, "BTC");
    assert_eq!(report.net_funding_pnl, net_funding_pnl);
    assert_eq!(report.payment_count, payments.len());
    assert!(!report.rates.is_empty());
    
    // Verify that payments were copied correctly
    assert_eq!(report.payments.len(), payments.len());
    for (i, payment) in payments.iter().enumerate() {
        assert_eq!(report.payments[i].timestamp, payment.timestamp);
        assert_eq!(report.payments[i].funding_rate, payment.funding_rate);
        assert_eq!(report.payments[i].position_size, payment.position_size);
        assert_eq!(report.payments[i].price, payment.price);
        assert_eq!(report.payments[i].payment_amount, payment.payment_amount);
    }
    
    // Verify that funding direction stats were calculated
    assert!(report.direction_stats.positive_count + report.direction_stats.negative_count > 0);
    
    // Verify that distribution stats were calculated
    assert!(report.distribution.total_periods > 0);
    
    // Verify that metrics by period were calculated
    assert!(!report.metrics_by_period.daily.is_empty());
    assert!(!report.metrics_by_period.weekly.is_empty());
    assert!(!report.metrics_by_period.monthly.is_empty());
    
    Ok(())
}

#[test]
fn test_funding_distribution_calculation() -> Result<()> {
    // Create test data with known funding rates
    let rates = vec![0.0001, 0.0002, 0.0003, -0.0001, -0.0002];
    
    // Calculate distribution
    let distribution = FundingReport::calculate_funding_distribution(&rates)?;
    
    // Verify distribution statistics
    assert_eq!(distribution.mean, rates.iter().sum::<f64>() / rates.len() as f64);
    assert_eq!(distribution.min, -0.0002);
    assert_eq!(distribution.max, 0.0003);
    
    // Median should be the middle value (sorted)
    assert_eq!(distribution.median, 0.0001);
    
    // Test with empty rates
    let empty_distribution = FundingReport::calculate_funding_distribution(&[])?;
    assert_eq!(empty_distribution.mean, 0.0);
    assert_eq!(empty_distribution.median, 0.0);
    assert_eq!(empty_distribution.std_dev, 0.0);
    
    Ok(())
}

#[test]
fn test_funding_direction_stats() {
    // Create test data with known funding rates
    let mut rates = Vec::new();
    
    // Add 5 positive rates
    for i in 0..5 {
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(1640995200 + i as i64 * 3600, 0).unwrap();
        rates.push(FundingRatePoint {
            timestamp,
            rate: 0.0001 * (i as f64 + 1.0),
        });
    }
    
    // Add 3 negative rates
    for i in 0..3 {
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(1641013200 + i as i64 * 3600, 0).unwrap();
        rates.push(FundingRatePoint {
            timestamp,
            rate: -0.0001 * (i as f64 + 1.0),
        });
    }
    
    // Add 2 zero rates
    for i in 0..2 {
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(1641024000 + i as i64 * 3600, 0).unwrap();
        rates.push(FundingRatePoint {
            timestamp,
            rate: 0.0,
        });
    }
    
    // Calculate direction stats
    let stats = FundingReport::calculate_direction_stats(&rates);
    
    // Verify stats
    assert_eq!(stats.positive_count, 5);
    assert_eq!(stats.negative_count, 3);
    assert_eq!(stats.zero_count, 2);
    assert_eq!(stats.positive_percentage, 0.5); // 5 out of 10
    assert_eq!(stats.negative_percentage, 0.3); // 3 out of 10
    
    // Calculate expected average positive rate
    let positive_sum = rates.iter()
        .filter(|r| r.rate > 0.0)
        .map(|r| r.rate)
        .sum::<f64>();
    assert_eq!(stats.avg_positive_rate, positive_sum / 5.0);
    
    // Calculate expected average negative rate
    let negative_sum = rates.iter()
        .filter(|r| r.rate < 0.0)
        .map(|r| r.rate)
        .sum::<f64>();
    assert_eq!(stats.avg_negative_rate, negative_sum / 3.0);
    
    // Test streak detection
    let mut streak_rates = Vec::new();
    
    // Add 3 positive rates
    for i in 0..3 {
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(1640995200 + i as i64 * 3600, 0).unwrap();
        streak_rates.push(FundingRatePoint {
            timestamp,
            rate: 0.0001,
        });
    }
    
    // Add 5 negative rates
    for i in 0..5 {
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(1641006000 + i as i64 * 3600, 0).unwrap();
        streak_rates.push(FundingRatePoint {
            timestamp,
            rate: -0.0001,
        });
    }
    
    // Add 2 positive rates
    for i in 0..2 {
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(1641024000 + i as i64 * 3600, 0).unwrap();
        streak_rates.push(FundingRatePoint {
            timestamp,
            rate: 0.0001,
        });
    }
    
    // Calculate direction stats
    let streak_stats = FundingReport::calculate_direction_stats(&streak_rates);
    
    // Verify streak stats
    assert_eq!(streak_stats.longest_positive_streak, 3);
    assert_eq!(streak_stats.longest_negative_streak, 5);
}

#[test]
fn test_funding_period_metrics() {
    // Create test data
    let data = generate_mock_data("BTC", 72, true, false);
    let positions = generate_position_sequence(data.len(), "constant_long");
    let payments = generate_mock_funding_payments(72, 1.0);
    let net_funding_pnl = payments.iter().map(|p| p.payment_amount).sum();
    
    // Create funding report
    let report = FundingReport::new(
        "BTC",
        &data,
        &positions,
        payments,
        net_funding_pnl,
    ).unwrap();
    
    // Verify period metrics
    assert_eq!(report.metrics_by_period.daily.len(), 1);
    assert_eq!(report.metrics_by_period.weekly.len(), 1);
    assert_eq!(report.metrics_by_period.monthly.len(), 1);
    
    // Daily metrics should have total PnL divided by 30
    assert!((report.metrics_by_period.daily[0].total_pnl - net_funding_pnl / 30.0).abs() < 0.0001);
    
    // Weekly metrics should have total PnL divided by 4
    assert!((report.metrics_by_period.weekly[0].total_pnl - net_funding_pnl / 4.0).abs() < 0.0001);
    
    // Monthly metrics should have total PnL
    assert!((report.metrics_by_period.monthly[0].total_pnl - net_funding_pnl).abs() < 0.0001);
}

#[test]
fn test_funding_report_with_empty_data() -> Result<()> {
    // Create empty data
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
    
    let positions = Vec::new();
    let payments = Vec::new();
    
    // Create funding report
    let report = FundingReport::new(
        "BTC",
        &data,
        &positions,
        payments,
        0.0,
    )?;
    
    // Verify report fields
    assert_eq!(report.symbol, "BTC");
    assert_eq!(report.net_funding_pnl, 0.0);
    assert_eq!(report.payment_count, 0);
    assert!(report.rates.is_empty());
    assert!(report.payments.is_empty());
    
    // Verify that distribution stats were calculated with defaults
    assert_eq!(report.distribution.total_periods, 0);
    assert_eq!(report.distribution.mean, 0.0);
    assert_eq!(report.distribution.median, 0.0);
    assert_eq!(report.distribution.std_dev, 0.0);
    
    // Verify that direction stats were calculated with defaults
    assert_eq!(report.direction_stats.positive_count, 0);
    assert_eq!(report.direction_stats.negative_count, 0);
    assert_eq!(report.direction_stats.zero_count, 0);
    
    Ok(())
}

#[test]
fn test_funding_report_with_all_positive_rates() -> Result<()> {
    // Create test data with all positive funding rates
    let mut data = generate_mock_data("BTC", 72, true, false);
    
    // Set all funding rates to positive values
    for i in 0..data.funding_rates.len() {
        if !data.funding_rates[i].is_nan() {
            data.funding_rates[i] = 0.0001 * (i as f64 % 5.0 + 1.0);
        }
    }
    
    let positions = generate_position_sequence(data.len(), "constant_long");
    let mut payments = generate_mock_funding_payments(72, 1.0);
    
    // Set all payments to negative (long pays when funding is positive)
    for payment in &mut payments {
        payment.funding_rate = payment.funding_rate.abs();
        payment.payment_amount = -payment.funding_rate * payment.price;
    }
    
    let net_funding_pnl = payments.iter().map(|p| p.payment_amount).sum();
    
    // Create funding report
    let report = FundingReport::new(
        "BTC",
        &data,
        &positions,
        payments,
        net_funding_pnl,
    )?;
    
    // Verify direction stats
    assert!(report.direction_stats.positive_count > 0);
    assert_eq!(report.direction_stats.negative_count, 0);
    assert_eq!(report.direction_stats.positive_percentage, 1.0);
    assert_eq!(report.direction_stats.negative_percentage, 0.0);
    
    // Net funding PnL should be negative (long pays when funding is positive)
    assert!(report.net_funding_pnl < 0.0);
    assert_eq!(report.total_funding_received, 0.0);
    assert!(report.total_funding_paid > 0.0);
    
    Ok(())
}

#[test]
fn test_funding_report_with_all_negative_rates() -> Result<()> {
    // Create test data with all negative funding rates
    let mut data = generate_mock_data("BTC", 72, true, false);
    
    // Set all funding rates to negative values
    for i in 0..data.funding_rates.len() {
        if !data.funding_rates[i].is_nan() {
            data.funding_rates[i] = -0.0001 * (i as f64 % 5.0 + 1.0);
        }
    }
    
    let positions = generate_position_sequence(data.len(), "constant_long");
    let mut payments = generate_mock_funding_payments(72, 1.0);
    
    // Set all payments to positive (long receives when funding is negative)
    for payment in &mut payments {
        payment.funding_rate = -payment.funding_rate.abs();
        payment.payment_amount = -payment.funding_rate * payment.price;
    }
    
    let net_funding_pnl = payments.iter().map(|p| p.payment_amount).sum();
    
    // Create funding report
    let report = FundingReport::new(
        "BTC",
        &data,
        &positions,
        payments,
        net_funding_pnl,
    )?;
    
    // Verify direction stats
    assert_eq!(report.direction_stats.positive_count, 0);
    assert!(report.direction_stats.negative_count > 0);
    assert_eq!(report.direction_stats.positive_percentage, 0.0);
    assert_eq!(report.direction_stats.negative_percentage, 1.0);
    
    // Net funding PnL should be positive (long receives when funding is negative)
    assert!(report.net_funding_pnl > 0.0);
    assert!(report.total_funding_received > 0.0);
    assert_eq!(report.total_funding_paid, 0.0);
    
    Ok(())
}

#[test]
fn test_funding_report_with_short_position() -> Result<()> {
    // Create test data
    let data = generate_mock_data("BTC", 72, true, false);
    let positions = generate_position_sequence(data.len(), "constant_short");
    let mut payments = generate_mock_funding_payments(72, -1.0); // Short position
    
    // Adjust payment amounts for short position
    for payment in &mut payments {
        payment.position_size = -1.0;
        payment.payment_amount = -payment.position_size * payment.funding_rate * payment.price;
    }
    
    let net_funding_pnl = payments.iter().map(|p| p.payment_amount).sum();
    
    // Create funding report
    let report = FundingReport::new(
        "BTC",
        &data,
        &positions,
        payments,
        net_funding_pnl,
    )?;
    
    // Verify report fields
    assert_eq!(report.symbol, "BTC");
    assert_eq!(report.net_funding_pnl, net_funding_pnl);
    
    // Short position should have opposite funding PnL compared to long
    // When funding is positive, short receives
    // When funding is negative, short pays
    
    Ok(())
}

#[test]
fn test_funding_rate_point_extraction() -> Result<()> {
    // Create test data with specific funding rates
    let mut data = generate_mock_data("BTC", 24, false, false);
    
    // Set funding rates at specific times
    for i in 0..24 {
        if i % 8 == 0 {
            data.funding_rates[i] = 0.0001 * (i as f64 / 8.0 + 1.0);
        } else {
            data.funding_rates[i] = f64::NAN;
        }
    }
    
    let positions = generate_position_sequence(data.len(), "constant_long");
    let payments = generate_mock_funding_payments(24, 1.0);
    let net_funding_pnl = payments.iter().map(|p| p.payment_amount).sum();
    
    // Create funding report
    let report = FundingReport::new(
        "BTC",
        &data,
        &positions,
        payments,
        net_funding_pnl,
    )?;
    
    // Verify that funding rate points were extracted correctly
    assert_eq!(report.rates.len(), 3); // Should have 3 funding rates (at hours 0, 8, 16)
    
    // Verify the rate values
    for i in 0..report.rates.len() {
        assert_eq!(report.rates[i].rate, 0.0001 * (i as f64 + 1.0));
    }
    
    Ok(())
}

#[test]
fn test_funding_report_with_alternating_positions() -> Result<()> {
    // Create test data
    let data = generate_mock_data("BTC", 72, true, false);
    
    // Create alternating positions (long, short, long, short, ...)
    let positions = generate_position_sequence(data.len(), "alternating");
    
    // Create payments with alternating positions
    let mut payments = Vec::new();
    let base_timestamp = 1640995200; // 2022-01-01 00:00:00 UTC
    
    for i in 0..9 { // 3 days * 3 payments per day
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(base_timestamp + i as i64 * 8 * 3600, 0).unwrap();
        
        let funding_rate = 0.0001 * ((i as f64 * 0.5).sin() + 0.5);
        let price = 100.0 + (i as f64);
        let position_size = if i % 2 == 0 { 1.0 } else { -1.0 };
        
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
    
    let net_funding_pnl = payments.iter().map(|p| p.payment_amount).sum();
    
    // Create funding report
    let report = FundingReport::new(
        "BTC",
        &data,
        &positions,
        payments,
        net_funding_pnl,
    )?;
    
    // Verify report fields
    assert_eq!(report.symbol, "BTC");
    assert_eq!(report.net_funding_pnl, net_funding_pnl);
    assert_eq!(report.payment_count, 9);
    
    Ok(())
}

#[test]
fn test_funding_report_with_zero_positions() -> Result<()> {
    // Create test data
    let data = generate_mock_data("BTC", 72, true, false);
    
    // Create zero positions
    let positions = generate_position_sequence(data.len(), "zero");
    
    // Create payments with zero positions
    let mut payments = Vec::new();
    let base_timestamp = 1640995200; // 2022-01-01 00:00:00 UTC
    
    for i in 0..9 { // 3 days * 3 payments per day
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(base_timestamp + i as i64 * 8 * 3600, 0).unwrap();
        
        let funding_rate = 0.0001 * ((i as f64 * 0.5).sin() + 0.5);
        let price = 100.0 + (i as f64);
        let position_size = 0.0;
        
        // Calculate payment amount (should be 0)
        let payment_amount = -position_size * funding_rate * price;
        
        payments.push(FundingPayment {
            timestamp,
            funding_rate,
            position_size,
            price,
            payment_amount,
        });
    }
    
    let net_funding_pnl = payments.iter().map(|p| p.payment_amount).sum();
    
    // Create funding report
    let report = FundingReport::new(
        "BTC",
        &data,
        &positions,
        payments,
        net_funding_pnl,
    )?;
    
    // Verify report fields
    assert_eq!(report.symbol, "BTC");
    assert_eq!(report.net_funding_pnl, 0.0); // Should be 0 with zero positions
    assert_eq!(report.payment_count, 9);
    assert_eq!(report.total_funding_received, 0.0);
    assert_eq!(report.total_funding_paid, 0.0);
    
    Ok(())
}