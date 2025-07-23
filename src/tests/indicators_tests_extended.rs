//! Extended tests for the indicators module

use crate::indicators::*;
use crate::errors::Result;
use chrono::{DateTime, FixedOffset, TimeZone};
use std::collections::VecDeque;

/// Test that funding direction is correctly determined from rates
#[test]
fn test_funding_direction_from_rate_standard() {
    // Test positive funding rate
    let direction = FundingDirection::from_rate(0.0001);
    assert_eq!(direction, FundingDirection::Positive);
    
    // Test negative funding rate
    let direction = FundingDirection::from_rate(-0.0001);
    assert_eq!(direction, FundingDirection::Negative);
    
    // Test zero funding rate
    let direction = FundingDirection::from_rate(0.0);
    assert_eq!(direction, FundingDirection::Neutral);
}

/// Test that funding volatility calculation works correctly
#[test]
fn test_funding_volatility_calculation_standard() {
    // Test with consistent funding rates
    let rates = vec![0.0001, 0.0001, 0.0001, 0.0001, 0.0001];
    let volatility = calculate_funding_volatility(&rates);
    assert_eq!(volatility, 0.0); // No volatility with constant rates
    
    // Test with varying funding rates
    let rates = vec![0.0001, 0.0002, 0.0003, 0.0002, 0.0001];
    let volatility = calculate_funding_volatility(&rates);
    assert!(volatility > 0.0); // Should have some volatility
}

/// Test that funding momentum calculation works correctly
#[test]
fn test_funding_momentum_calculation_standard() {
    // Test with increasing funding rates
    let rates = vec![0.0001, 0.0002, 0.0003, 0.0004, 0.0005];
    let momentum = calculate_funding_momentum(&rates);
    assert!(momentum > 0.0); // Positive momentum
    
    // Test with decreasing funding rates
    let rates = vec![0.0005, 0.0004, 0.0003, 0.0002, 0.0001];
    let momentum = calculate_funding_momentum(&rates);
    assert!(momentum < 0.0); // Negative momentum
}

/// Test that funding arbitrage calculation works correctly
#[test]
fn test_funding_arbitrage_calculation_standard() {
    // Test with positive funding rate
    let funding_rate = 0.0005;
    let price = 50000.0;
    let opportunity = calculate_funding_arbitrage(funding_rate, price);
    
    assert!(opportunity.is_arbitrage);
    assert_eq!(opportunity.direction, FundingDirection::Positive);
    
    // Test with negative funding rate
    let funding_rate = -0.0005;
    let price = 50000.0;
    let opportunity = calculate_funding_arbitrage(funding_rate, price);
    
    assert!(opportunity.is_arbitrage);
    assert_eq!(opportunity.direction, FundingDirection::Negative);
}

/// Test that basis indicator calculation works correctly
#[test]
fn test_basis_indicator_calculation_standard() {
    // Test with positive basis
    let spot_price = 50000.0;
    let futures_price = 50500.0;
    let days_to_expiry = 30.0;
    
    let basis = calculate_basis_indicator(spot_price, futures_price, days_to_expiry);
    
    assert!(basis.basis > 0.0); // Positive basis (contango)
    assert!(basis.annualized_basis > 0.0);
    
    // Test with negative basis
    let spot_price = 50000.0;
    let futures_price = 49500.0;
    let days_to_expiry = 30.0;
    
    let basis = calculate_basis_indicator(spot_price, futures_price, days_to_expiry);
    
    assert!(basis.basis < 0.0); // Negative basis (backwardation)
    assert!(basis.annualized_basis < 0.0);
}

/// Test that FundingPredictionConfig works correctly
#[test]
fn test_funding_prediction_config() {
    // Test default config
    let default_config = FundingPredictionConfig::default();
    
    assert_eq!(default_config.lookback_periods, 48);
    
    // Weights should sum to 1.0
    let sum = default_config.volatility_weight + 
              default_config.momentum_weight + 
              default_config.basis_weight + 
              default_config.correlation_weight;
    
    assert!((sum - 1.0).abs() < 0.0001);
    
    // Test custom config
    let custom_config = FundingPredictionConfig {
        lookback_periods: 24,
        volatility_weight: 0.1,
        momentum_weight: 0.2,
        basis_weight: 0.3,
        correlation_weight: 0.4,
    };
    
    assert_eq!(custom_config.lookback_periods, 24);
    assert_eq!(custom_config.volatility_weight, 0.1);
    assert_eq!(custom_config.momentum_weight, 0.2);
    assert_eq!(custom_config.basis_weight, 0.3);
    assert_eq!(custom_config.correlation_weight, 0.4);
}

/// Test that FundingRatePredictor works correctly
#[test]
fn test_funding_rate_predictor() {
    // Create a predictor with custom config
    let config = FundingPredictionConfig {
        lookback_periods: 10,
        volatility_weight: 0.25,
        momentum_weight: 0.25,
        basis_weight: 0.25,
        correlation_weight: 0.25,
    };
    
    let mut predictor = FundingRatePredictor::new(config);
    
    // Test with empty predictor
    let prediction = predictor.predict();
    assert_eq!(prediction.expected_rate, 0.0);
    assert_eq!(prediction.direction, FundingDirection::Neutral);
    assert_eq!(prediction.confidence, 0.0);
    
    // Add increasing observations
    for i in 0..10 {
        predictor.add_observation(0.0001 * i as f64);
    }
    
    // Test prediction with increasing trend
    let prediction = predictor.predict();
    assert!(prediction.expected_rate > 0.0009); // Should predict continued increase
    assert_eq!(prediction.direction, FundingDirection::Positive);
    assert!(prediction.confidence > 0.5); // Should have high confidence
    
    // Test volatility calculation
    let volatility = predictor.get_volatility();
    assert!(volatility > 0.0);
    
    // Test momentum calculation
    let momentum = predictor.get_momentum();
    assert!(momentum > 0.0);
    
    // Test with decreasing trend
    let mut predictor = FundingRatePredictor::new(config);
    for i in 0..10 {
        predictor.add_observation(0.001 - 0.0001 * i as f64);
    }
    
    let prediction = predictor.predict();
    assert!(prediction.expected_rate < 0.0001); // Should predict continued decrease
    assert_eq!(prediction.direction, FundingDirection::Positive); // Still positive but decreasing
    
    // Test with alternating values
    let mut predictor = FundingRatePredictor::new(config);
    for i in 0..10 {
        predictor.add_observation(if i % 2 == 0 { 0.0001 } else { -0.0001 });
    }
    
    let prediction = predictor.predict();
    assert!(prediction.confidence < 0.7); // Should have lower confidence with alternating values
}

/// Test that FundingRatePredictor correlation works correctly
#[test]
fn test_funding_rate_predictor_correlation() {
    // Create two predictors
    let config = FundingPredictionConfig::default();
    let mut predictor1 = FundingRatePredictor::new(config.clone());
    let mut predictor2 = FundingRatePredictor::new(config.clone());
    
    // Add identical observations to both
    for i in 0..10 {
        let rate = 0.0001 * i as f64;
        predictor1.add_observation(rate);
        predictor2.add_observation(rate);
    }
    
    // Test correlation with self (should be 1.0)
    let correlation = predictor1.correlation_with(&predictor1);
    assert!((correlation - 1.0).abs() < 0.0001);
    
    // Test correlation with identical predictor (should be 1.0)
    let correlation = predictor1.correlation_with(&predictor2);
    assert!((correlation - 1.0).abs() < 0.0001);
    
    // Create predictor with opposite trend
    let mut predictor3 = FundingRatePredictor::new(config);
    for i in 0..10 {
        let rate = 0.0001 * (9 - i) as f64;
        predictor3.add_observation(rate);
    }
    
    // Test correlation with opposite trend (should be negative)
    let correlation = predictor1.correlation_with(&predictor3);
    assert!(correlation < 0.0);
    
    // Test correlation with empty predictor
    let predictor4 = FundingRatePredictor::new(config);
    let correlation = predictor1.correlation_with(&predictor4);
    assert_eq!(correlation, 0.0);
}

/// Test that FundingRatePredictor anomaly detection works correctly
#[test]
fn test_funding_rate_predictor_anomaly_detection() {
    // Create a predictor with custom config
    let config = FundingPredictionConfig {
        lookback_periods: 10,
        volatility_weight: 0.25,
        momentum_weight: 0.25,
        basis_weight: 0.25,
        correlation_weight: 0.25,
    };
    
    let mut predictor = FundingRatePredictor::new(config);
    
    // Add normal observations
    for _ in 0..9 {
        predictor.add_observation(0.0001);
    }
    
    // Test with normal observation
    predictor.add_observation(0.0001);
    let anomaly = predictor.detect_anomaly();
    assert!(!anomaly.is_anomaly);
    
    // Test with slightly abnormal observation
    predictor.add_observation(0.0002);
    let anomaly = predictor.detect_anomaly();
    assert!(!anomaly.is_anomaly); // Should not be flagged as anomaly
    
    // Test with highly abnormal observation
    predictor.add_observation(0.001); // 10x higher
    let anomaly = predictor.detect_anomaly();
    assert!(anomaly.is_anomaly);
    assert!(anomaly.deviation > 3.0); // Should be more than 3 sigma
    assert_eq!(anomaly.direction, FundingDirection::Positive);
    
    // Test with highly abnormal negative observation
    predictor.add_observation(-0.001); // Much lower
    let anomaly = predictor.detect_anomaly();
    assert!(anomaly.is_anomaly);
    assert!(anomaly.deviation > 3.0);
    assert_eq!(anomaly.direction, FundingDirection::Negative);
}

/// Test that FundingRatePredictor cycle detection works correctly
#[test]
fn test_funding_rate_predictor_cycle_detection() {
    // Create a predictor with custom config
    let config = FundingPredictionConfig {
        lookback_periods: 24,
        volatility_weight: 0.25,
        momentum_weight: 0.25,
        basis_weight: 0.25,
        correlation_weight: 0.25,
    };
    
    let mut predictor = FundingRatePredictor::new(config);
    
    // Add observations with a clear 8-hour cycle
    for i in 0..24 {
        let hour = i % 8;
        let rate = match hour {
            0 => 0.0003, // Peak at 00:00
            1 => 0.0002,
            2 => 0.0001,
            3 => 0.0,
            4 => -0.0001, // Trough at 04:00
            5 => 0.0,
            6 => 0.0001,
            7 => 0.0002,
            _ => unreachable!(),
        };
        predictor.add_observation(rate);
    }
    
    // Detect funding cycle
    let cycle = predictor.detect_funding_cycle();
    
    // Should detect an 8-hour cycle
    assert_eq!(cycle.period_hours, 8);
    assert!(cycle.strength > 0.5); // Should have strong cycle detection
    assert!(cycle.is_significant);
    
    // Test with random data (should not detect a strong cycle)
    let mut predictor = FundingRatePredictor::new(config);
    for i in 0..24 {
        let rate = 0.0001 * ((i as f64 * 0.123).sin() + (i as f64 * 0.456).cos());
        predictor.add_observation(rate);
    }
    
    let cycle = predictor.detect_funding_cycle();
    assert!(cycle.strength < 0.7); // Should have weaker cycle detection
}

/// Test that OpenInterestChange works correctly
#[test]
fn test_open_interest_change() {
    // Test increasing open interest
    let prev_oi = 1000.0;
    let curr_oi = 1100.0;
    let price = 50000.0;
    
    let change = OpenInterestChange::new(prev_oi, curr_oi, price);
    
    assert_eq!(change.absolute_change, 100.0);
    assert_eq!(change.percentage_change, 0.1); // 10% increase
    assert_eq!(change.usd_value_change, 100.0 * 50000.0);
    assert!(change.is_increasing);
    assert!(!change.is_decreasing);
    
    // Test decreasing open interest
    let prev_oi = 1000.0;
    let curr_oi = 900.0;
    let price = 50000.0;
    
    let change = OpenInterestChange::new(prev_oi, curr_oi, price);
    
    assert_eq!(change.absolute_change, -100.0);
    assert_eq!(change.percentage_change, -0.1); // 10% decrease
    assert_eq!(change.usd_value_change, -100.0 * 50000.0);
    assert!(!change.is_increasing);
    assert!(change.is_decreasing);
    
    // Test unchanged open interest
    let prev_oi = 1000.0;
    let curr_oi = 1000.0;
    let price = 50000.0;
    
    let change = OpenInterestChange::new(prev_oi, curr_oi, price);
    
    assert_eq!(change.absolute_change, 0.0);
    assert_eq!(change.percentage_change, 0.0);
    assert_eq!(change.usd_value_change, 0.0);
    assert!(!change.is_increasing);
    assert!(!change.is_decreasing);
    
    // Test with zero previous OI
    let prev_oi = 0.0;
    let curr_oi = 1000.0;
    let price = 50000.0;
    
    let change = OpenInterestChange::new(prev_oi, curr_oi, price);
    
    assert_eq!(change.absolute_change, 1000.0);
    assert_eq!(change.percentage_change, 0.0); // Should handle division by zero
    assert_eq!(change.usd_value_change, 1000.0 * 50000.0);
    assert!(change.is_increasing);
    assert!(!change.is_decreasing);
}

/// Test that LiquidationImpact works correctly
#[test]
fn test_liquidation_impact() {
    // Test significant liquidation
    let liquidation_amount = 1000.0;
    let open_interest = 10000.0;
    let price = 50000.0;
    let price_impact = -0.02; // 2% price drop
    
    let impact = LiquidationImpact::new(liquidation_amount, open_interest, price, price_impact);
    
    assert_eq!(impact.liquidation_percentage, 0.1); // 10% of OI
    assert_eq!(impact.usd_value, liquidation_amount * price);
    assert_eq!(impact.price_impact, price_impact);
    assert!(impact.is_significant);
    
    // Test minor liquidation
    let liquidation_amount = 100.0;
    let open_interest = 10000.0;
    let price = 50000.0;
    let price_impact = -0.001; // 0.1% price drop
    
    let impact = LiquidationImpact::new(liquidation_amount, open_interest, price, price_impact);
    
    assert_eq!(impact.liquidation_percentage, 0.01); // 1% of OI
    assert_eq!(impact.usd_value, liquidation_amount * price);
    assert_eq!(impact.price_impact, price_impact);
    assert!(!impact.is_significant); // Below default threshold
    
    // Test with zero open interest
    let liquidation_amount = 1000.0;
    let open_interest = 0.0;
    let price = 50000.0;
    let price_impact = -0.02;
    
    let impact = LiquidationImpact::new(liquidation_amount, open_interest, price, price_impact);
    
    assert_eq!(impact.liquidation_percentage, 0.0); // Should handle division by zero
    assert_eq!(impact.usd_value, liquidation_amount * price);
    assert_eq!(impact.price_impact, price_impact);
    assert!(impact.is_significant); // Still significant due to price impact
}

/// Test that FundingPriceCorrelation works correctly
#[test]
fn test_funding_price_correlation() {
    // Test positive correlation
    let funding_rates = vec![0.0001, 0.0002, 0.0003, 0.0004, 0.0005];
    let prices = vec![50000.0, 50100.0, 50200.0, 50300.0, 50400.0];
    
    let correlation = FundingPriceCorrelation::calculate(&funding_rates, &prices);
    
    assert!(correlation.coefficient > 0.9); // Strong positive correlation
    assert!(correlation.is_significant);
    assert_eq!(correlation.relationship, "Positive");
    
    // Test negative correlation
    let funding_rates = vec![0.0001, 0.0002, 0.0003, 0.0004, 0.0005];
    let prices = vec![50400.0, 50300.0, 50200.0, 50100.0, 50000.0];
    
    let correlation = FundingPriceCorrelation::calculate(&funding_rates, &prices);
    
    assert!(correlation.coefficient < -0.9); // Strong negative correlation
    assert!(correlation.is_significant);
    assert_eq!(correlation.relationship, "Negative");
    
    // Test no correlation
    let funding_rates = vec![0.0001, 0.0002, 0.0001, 0.0002, 0.0001];
    let prices = vec![50000.0, 50100.0, 50000.0, 50100.0, 50000.0];
    
    let correlation = FundingPriceCorrelation::calculate(&funding_rates, &prices);
    
    assert!(correlation.coefficient.abs() < 0.5); // Weak correlation
    assert!(!correlation.is_significant);
    assert_eq!(correlation.relationship, "Weak");
    
    // Test with empty arrays
    let funding_rates: Vec<f64> = Vec::new();
    let prices: Vec<f64> = Vec::new();
    
    let correlation = FundingPriceCorrelation::calculate(&funding_rates, &prices);
    
    assert_eq!(correlation.coefficient, 0.0);
    assert!(!correlation.is_significant);
    assert_eq!(correlation.relationship, "Unknown");
    
    // Test with different length arrays
    let funding_rates = vec![0.0001, 0.0002, 0.0003];
    let prices = vec![50000.0, 50100.0, 50200.0, 50300.0, 50400.0];
    
    let correlation = FundingPriceCorrelation::calculate(&funding_rates, &prices);
    
    assert_eq!(correlation.coefficient, 0.0);
    assert!(!correlation.is_significant);
    assert_eq!(correlation.relationship, "Unknown");
}

/// Test that AsAny trait works correctly
#[test]
fn test_as_any_trait() {
    // Create a predictor
    let config = FundingPredictionConfig::default();
    let predictor = FundingRatePredictor::new(config);
    
    // Get as Any
    let any = predictor.as_any();
    
    // Downcast back to FundingRatePredictor
    let downcast = any.downcast_ref::<FundingRatePredictor>();
    assert!(downcast.is_some());
    
    // Downcast to wrong type
    let wrong_downcast = any.downcast_ref::<String>();
    assert!(wrong_downcast.is_none());
}