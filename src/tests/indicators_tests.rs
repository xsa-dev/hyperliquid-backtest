//! Tests for the indicators module

use crate::indicators::*;
use crate::errors::Result;
use chrono::{DateTime, FixedOffset, TimeZone};
use std::collections::VecDeque;

#[test]
fn test_funding_direction_from_rate() {
    // Test positive funding rate
    let direction = FundingDirection::from_rate(0.0001);
    assert_eq!(direction, FundingDirection::Positive);
    
    // Test negative funding rate
    let direction = FundingDirection::from_rate(-0.0001);
    assert_eq!(direction, FundingDirection::Negative);
    
    // Test zero funding rate
    let direction = FundingDirection::from_rate(0.0);
    assert_eq!(direction, FundingDirection::Neutral);
    
    // Test very small positive funding rate
    let direction = FundingDirection::from_rate(0.000001);
    assert_eq!(direction, FundingDirection::Positive);
    
    // Test very small negative funding rate
    let direction = FundingDirection::from_rate(-0.000001);
    assert_eq!(direction, FundingDirection::Negative);
}

#[test]
fn test_funding_volatility_calculation() {
    // Test with consistent funding rates
    let rates = vec![0.0001, 0.0001, 0.0001, 0.0001, 0.0001];
    let volatility = calculate_funding_volatility(&rates);
    assert_eq!(volatility, 0.0); // No volatility with constant rates
    
    // Test with varying funding rates
    let rates = vec![0.0001, 0.0002, 0.0003, 0.0002, 0.0001];
    let volatility = calculate_funding_volatility(&rates);
    assert!(volatility > 0.0); // Should have some volatility
    
    // Test with alternating positive and negative rates
    let rates = vec![0.0001, -0.0001, 0.0001, -0.0001, 0.0001];
    let volatility = calculate_funding_volatility(&rates);
    assert!(volatility > 0.0); // Should have higher volatility
    
    // Test with empty rates
    let rates: Vec<f64> = Vec::new();
    let volatility = calculate_funding_volatility(&rates);
    assert_eq!(volatility, 0.0); // Should return 0 for empty input
    
    // Test with single rate
    let rates = vec![0.0001];
    let volatility = calculate_funding_volatility(&rates);
    assert_eq!(volatility, 0.0); // Should return 0 for single input
}

#[test]
fn test_funding_momentum_calculation() {
    // Test with increasing funding rates
    let rates = vec![0.0001, 0.0002, 0.0003, 0.0004, 0.0005];
    let momentum = calculate_funding_momentum(&rates);
    assert!(momentum > 0.0); // Positive momentum
    
    // Test with decreasing funding rates
    let rates = vec![0.0005, 0.0004, 0.0003, 0.0002, 0.0001];
    let momentum = calculate_funding_momentum(&rates);
    assert!(momentum < 0.0); // Negative momentum
    
    // Test with flat funding rates
    let rates = vec![0.0001, 0.0001, 0.0001, 0.0001, 0.0001];
    let momentum = calculate_funding_momentum(&rates);
    assert_eq!(momentum, 0.0); // No momentum
    
    // Test with alternating rates
    let rates = vec![0.0001, 0.0002, 0.0001, 0.0002, 0.0001];
    let momentum = calculate_funding_momentum(&rates);
    assert_eq!(momentum, 0.0); // No clear momentum
    
    // Test with empty rates
    let rates: Vec<f64> = Vec::new();
    let momentum = calculate_funding_momentum(&rates);
    assert_eq!(momentum, 0.0); // Should return 0 for empty input
    
    // Test with single rate
    let rates = vec![0.0001];
    let momentum = calculate_funding_momentum(&rates);
    assert_eq!(momentum, 0.0); // Should return 0 for single input
}

#[test]
fn test_funding_arbitrage_calculation() {
    // Test with positive funding rate
    let funding_rate = 0.0005; // 0.05% per 8h
    let price = 50000.0;
    let opportunity = calculate_funding_arbitrage(funding_rate, price);
    
    assert!(opportunity.is_arbitrage);
    assert_eq!(opportunity.direction, FundingDirection::Positive);
    assert_eq!(opportunity.annualized_yield, funding_rate * 3 * 365.25); // 3 funding periods per day
    assert_eq!(opportunity.payment_per_contract, funding_rate * price);
    
    // Test with negative funding rate
    let funding_rate = -0.0005; // -0.05% per 8h
    let price = 50000.0;
    let opportunity = calculate_funding_arbitrage(funding_rate, price);
    
    assert!(opportunity.is_arbitrage);
    assert_eq!(opportunity.direction, FundingDirection::Negative);
    assert_eq!(opportunity.annualized_yield, -funding_rate * 3 * 365.25); // 3 funding periods per day
    assert_eq!(opportunity.payment_per_contract, -funding_rate * price);
    
    // Test with small funding rate (below threshold)
    let funding_rate = 0.00001; // 0.001% per 8h
    let price = 50000.0;
    let opportunity = calculate_funding_arbitrage(funding_rate, price);
    
    assert!(!opportunity.is_arbitrage); // Too small to be considered arbitrage
    assert_eq!(opportunity.direction, FundingDirection::Positive);
    
    // Test with zero funding rate
    let funding_rate = 0.0;
    let price = 50000.0;
    let opportunity = calculate_funding_arbitrage(funding_rate, price);
    
    assert!(!opportunity.is_arbitrage);
    assert_eq!(opportunity.direction, FundingDirection::Neutral);
    assert_eq!(opportunity.annualized_yield, 0.0);
    assert_eq!(opportunity.payment_per_contract, 0.0);
}

#[test]
fn test_basis_indicator_calculation() {
    // Test with positive basis (spot price > futures price)
    let spot_price = 50000.0;
    let futures_price = 49500.0;
    let days_to_expiry = 30.0;
    
    let basis = calculate_basis_indicator(spot_price, futures_price, days_to_expiry);
    
    assert!(basis.basis < 0.0); // Negative basis (backwardation)
    assert!(basis.annualized_basis < 0.0);
    assert_eq!(basis.basis_amount, futures_price - spot_price);
    
    // Test with negative basis (futures price > spot price)
    let spot_price = 50000.0;
    let futures_price = 50500.0;
    let days_to_expiry = 30.0;
    
    let basis = calculate_basis_indicator(spot_price, futures_price, days_to_expiry);
    
    assert!(basis.basis > 0.0); // Positive basis (contango)
    assert!(basis.annualized_basis > 0.0);
    assert_eq!(basis.basis_amount, futures_price - spot_price);
    
    // Test with zero basis
    let spot_price = 50000.0;
    let futures_price = 50000.0;
    let days_to_expiry = 30.0;
    
    let basis = calculate_basis_indicator(spot_price, futures_price, days_to_expiry);
    
    assert_eq!(basis.basis, 0.0);
    assert_eq!(basis.annualized_basis, 0.0);
    assert_eq!(basis.basis_amount, 0.0);
    
    // Test with different days to expiry
    let spot_price = 50000.0;
    let futures_price = 50500.0;
    let days_to_expiry = 90.0;
    
    let basis = calculate_basis_indicator(spot_price, futures_price, days_to_expiry);
    let basis_30_days = calculate_basis_indicator(spot_price, futures_price, 30.0);
    
    // Annualized basis should be the same regardless of days to expiry
    assert!((basis.annualized_basis - basis_30_days.annualized_basis).abs() < 0.0001);
    
    // But the raw basis should be different
    assert_eq!(basis.basis, (futures_price - spot_price) / spot_price);
    assert_eq!(basis_30_days.basis, (futures_price - spot_price) / spot_price);
}

#[test]
fn test_funding_prediction_config() {
    let config = FundingPredictionConfig {
        lookback_periods: 24,
        volatility_weight: 0.2,
        momentum_weight: 0.3,
        basis_weight: 0.3,
        correlation_weight: 0.2,
    };
    
    assert_eq!(config.lookback_periods, 24);
    assert_eq!(config.volatility_weight, 0.2);
    assert_eq!(config.momentum_weight, 0.3);
    assert_eq!(config.basis_weight, 0.3);
    assert_eq!(config.correlation_weight, 0.2);
    
    // Test default config
    let default_config = FundingPredictionConfig::default();
    
    assert_eq!(default_config.lookback_periods, 48); // Default is 48 periods
    assert!(default_config.volatility_weight > 0.0);
    assert!(default_config.momentum_weight > 0.0);
    assert!(default_config.basis_weight > 0.0);
    assert!(default_config.correlation_weight > 0.0);
    
    // Weights should sum to 1.0
    let sum = default_config.volatility_weight + 
              default_config.momentum_weight + 
              default_config.basis_weight + 
              default_config.correlation_weight;
    
    assert!((sum - 1.0).abs() < 0.0001);
}

#[test]
fn test_funding_rate_predictor() {
    // Create a predictor with custom config
    let config = FundingPredictionConfig {
        lookback_periods: 5,
        volatility_weight: 0.25,
        momentum_weight: 0.25,
        basis_weight: 0.25,
        correlation_weight: 0.25,
    };
    
    let mut predictor = FundingRatePredictor::new(config);
    
    // Add observations
    predictor.add_observation(0.0001);
    predictor.add_observation(0.0002);
    predictor.add_observation(0.0003);
    predictor.add_observation(0.0004);
    predictor.add_observation(0.0005);
    
    // Test prediction
    let prediction = predictor.predict();
    
    // With increasing funding rates, should predict continued increase
    assert_eq!(prediction.direction, FundingDirection::Positive);
    assert!(prediction.confidence > 0.5); // Should have high confidence
    assert!(prediction.expected_rate > 0.0);
    
    // Test volatility calculation
    let volatility = predictor.get_volatility();
    assert!(volatility > 0.0);
    
    // Test momentum calculation
    let momentum = predictor.get_momentum();
    assert!(momentum > 0.0);
    
    // Test with decreasing rates
    let mut predictor = FundingRatePredictor::new(config);
    predictor.add_observation(0.0005);
    predictor.add_observation(0.0004);
    predictor.add_observation(0.0003);
    predictor.add_observation(0.0002);
    predictor.add_observation(0.0001);
    
    let prediction = predictor.predict();
    
    // With decreasing funding rates, should predict continued decrease
    assert_eq!(prediction.direction, FundingDirection::Positive); // Still positive rate
    assert!(prediction.expected_rate < 0.0001); // But decreasing
    
    // Test with alternating rates
    let mut predictor = FundingRatePredictor::new(config);
    predictor.add_observation(0.0001);
    predictor.add_observation(-0.0001);
    predictor.add_observation(0.0001);
    predictor.add_observation(-0.0001);
    predictor.add_observation(0.0001);
    
    let prediction = predictor.predict();
    
    // With alternating funding rates, confidence should be lower
    assert!(prediction.confidence < 0.7);
}

#[test]
fn test_funding_rate_predictor_correlation() {
    // Create two predictors
    let config = FundingPredictionConfig::default();
    let mut predictor1 = FundingRatePredictor::new(config.clone());
    let mut predictor2 = FundingRatePredictor::new(config);
    
    // Add identical observations to both
    for i in 0..10 {
        let rate = 0.0001 * (i as f64);
        predictor1.add_observation(rate);
        predictor2.add_observation(rate);
    }
    
    // Correlation should be 1.0 for identical data
    let correlation = predictor1.correlation_with(&predictor2);
    assert!((correlation - 1.0).abs() < 0.0001);
    
    // Add different observations to second predictor
    for i in 0..5 {
        let rate = -0.0001 * (i as f64);
        predictor2.add_observation(rate);
    }
    
    // Correlation should be less than 1.0 now
    let correlation = predictor1.correlation_with(&predictor2);
    assert!(correlation < 1.0);
    
    // Create predictor with opposite trend
    let mut predictor3 = FundingRatePredictor::new(config);
    for i in 0..10 {
        let rate = -0.0001 * (i as f64);
        predictor3.add_observation(rate);
    }
    
    // Correlation should be negative for opposite trends
    let correlation = predictor1.correlation_with(&predictor3);
    assert!(correlation < 0.0);
}

#[test]
fn test_funding_cycle_detection() {
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
}

#[test]
fn test_funding_anomaly_detection() {
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
    
    // Add anomalous observation
    predictor.add_observation(0.001); // 10x higher than normal
    
    // Detect anomaly
    let anomaly = predictor.detect_anomaly();
    
    // Should detect the anomaly
    assert!(anomaly.is_anomaly);
    assert!(anomaly.deviation > 5.0); // Should be significantly higher than normal
    assert_eq!(anomaly.direction, FundingDirection::Positive);
    
    // Test with negative anomaly
    let mut predictor = FundingRatePredictor::new(config);
    
    // Add normal observations
    for _ in 0..9 {
        predictor.add_observation(0.0001);
    }
    
    // Add anomalous observation
    predictor.add_observation(-0.001); // Much lower than normal
    
    // Detect anomaly
    let anomaly = predictor.detect_anomaly();
    
    // Should detect the anomaly
    assert!(anomaly.is_anomaly);
    assert!(anomaly.deviation > 5.0);
    assert_eq!(anomaly.direction, FundingDirection::Negative);
}

#[test]
fn test_open_interest_change() {
    // Test increasing open interest
    let prev_oi = 1000.0;
    let curr_oi = 1100.0;
    let price = 50000.0;
    
    let change = OpenInterestChange::new(prev_oi, curr_oi, price);
    
    assert_eq!(change.absolute_change, 100.0);
    assert_eq!(change.percentage_change, 0.1); // 10% increase
    assert!(change.is_increasing);
    assert!(!change.is_decreasing);
    
    // Test decreasing open interest
    let prev_oi = 1000.0;
    let curr_oi = 900.0;
    let price = 50000.0;
    
    let change = OpenInterestChange::new(prev_oi, curr_oi, price);
    
    assert_eq!(change.absolute_change, -100.0);
    assert_eq!(change.percentage_change, -0.1); // 10% decrease
    assert!(!change.is_increasing);
    assert!(change.is_decreasing);
    
    // Test unchanged open interest
    let prev_oi = 1000.0;
    let curr_oi = 1000.0;
    let price = 50000.0;
    
    let change = OpenInterestChange::new(prev_oi, curr_oi, price);
    
    assert_eq!(change.absolute_change, 0.0);
    assert_eq!(change.percentage_change, 0.0);
    assert!(!change.is_increasing);
    assert!(!change.is_decreasing);
    
    // Test with price change
    let prev_oi = 1000.0;
    let curr_oi = 1100.0;
    let price = 55000.0; // 10% higher price
    
    let change = OpenInterestChange::new(prev_oi, curr_oi, price);
    
    // USD value should account for both OI and price change
    assert_eq!(change.usd_value_change, (curr_oi - prev_oi) * price);
}

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
}

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
    
    // Test with empty arrays
    let funding_rates: Vec<f64> = Vec::new();
    let prices: Vec<f64> = Vec::new();
    
    let correlation = FundingPriceCorrelation::calculate(&funding_rates, &prices);
    
    assert_eq!(correlation.coefficient, 0.0);
    assert!(!correlation.is_significant);
    
    // Test with different length arrays
    let funding_rates = vec![0.0001, 0.0002, 0.0003];
    let prices = vec![50000.0, 50100.0, 50200.0, 50300.0, 50400.0];
    
    let correlation = FundingPriceCorrelation::calculate(&funding_rates, &prices);
    
    assert_eq!(correlation.coefficient, 0.0);
    assert!(!correlation.is_significant);
}