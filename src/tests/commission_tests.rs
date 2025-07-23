//! Tests for the commission-related functionality

use crate::backtest::{
    HyperliquidCommission, OrderType, TradingScenario, 
    CommissionTracker, OrderTypeStrategy
};
use crate::errors::Result;
use chrono::{DateTime, FixedOffset, TimeZone};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[test]
fn test_hyperliquid_commission_default() {
    let commission = HyperliquidCommission::default();
    
    assert_eq!(commission.maker_rate, 0.0002); // 0.02%
    assert_eq!(commission.taker_rate, 0.0005); // 0.05%
    assert!(commission.funding_enabled);
}

#[test]
fn test_hyperliquid_commission_new() {
    let commission = HyperliquidCommission::new(0.0001, 0.0003, false);
    
    assert_eq!(commission.maker_rate, 0.0001);
    assert_eq!(commission.taker_rate, 0.0003);
    assert!(!commission.funding_enabled);
}

#[test]
fn test_calculate_fee() {
    let commission = HyperliquidCommission::default();
    let trade_value = 10000.0;
    
    let maker_fee = commission.calculate_fee(OrderType::LimitMaker, trade_value);
    let market_fee = commission.calculate_fee(OrderType::Market, trade_value);
    let limit_taker_fee = commission.calculate_fee(OrderType::LimitTaker, trade_value);
    
    assert_eq!(maker_fee, trade_value * 0.0002);
    assert_eq!(market_fee, trade_value * 0.0005);
    assert_eq!(limit_taker_fee, trade_value * 0.0005);
}

#[test]
fn test_calculate_scenario_fee() {
    let commission = HyperliquidCommission::default();
    let trade_value = 10000.0;
    
    // Test different scenarios with different order types
    let open_market_fee = commission.calculate_scenario_fee(
        TradingScenario::OpenPosition, OrderType::Market, trade_value
    );
    let close_maker_fee = commission.calculate_scenario_fee(
        TradingScenario::ClosePosition, OrderType::LimitMaker, trade_value
    );
    let reduce_taker_fee = commission.calculate_scenario_fee(
        TradingScenario::ReducePosition, OrderType::LimitTaker, trade_value
    );
    let increase_market_fee = commission.calculate_scenario_fee(
        TradingScenario::IncreasePosition, OrderType::Market, trade_value
    );
    
    // Verify fees are calculated correctly
    assert_eq!(open_market_fee, trade_value * 0.0005);
    assert_eq!(close_maker_fee, trade_value * 0.0002);
    assert_eq!(reduce_taker_fee, trade_value * 0.0005);
    assert_eq!(increase_market_fee, trade_value * 0.0005);
}

#[test]
fn test_to_rs_backtester_commission() {
    let commission = HyperliquidCommission::default();
    let rs_commission = commission.to_rs_backtester_commission();
    
    // Verify the rs-backtester commission uses the taker rate
    assert_eq!(rs_commission.rate, commission.taker_rate);
}

#[test]
fn test_validate_valid_commission() -> Result<()> {
    let commission = HyperliquidCommission::default();
    let result = commission.validate();
    
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_validate_invalid_maker_rate() {
    let commission = HyperliquidCommission::new(-0.1, 0.0005, true);
    let result = commission.validate();
    
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Invalid maker rate"));
    }
    
    let commission = HyperliquidCommission::new(1.5, 0.0005, true);
    let result = commission.validate();
    
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Invalid maker rate"));
    }
}

#[test]
fn test_validate_invalid_taker_rate() {
    let commission = HyperliquidCommission::new(0.0002, -0.1, true);
    let result = commission.validate();
    
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Invalid taker rate"));
    }
    
    let commission = HyperliquidCommission::new(0.0002, 1.5, true);
    let result = commission.validate();
    
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Invalid taker rate"));
    }
}

#[test]
fn test_validate_maker_higher_than_taker() {
    let commission = HyperliquidCommission::new(0.0006, 0.0005, true);
    let result = commission.validate();
    
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Maker rate should typically be lower than taker rate"));
    }
}

#[test]
fn test_order_type_strategy_always_market() {
    let strategy = OrderTypeStrategy::AlwaysMarket;
    
    for i in 0..10 {
        assert_eq!(strategy.get_order_type(i), OrderType::Market);
    }
}

#[test]
fn test_order_type_strategy_always_maker() {
    let strategy = OrderTypeStrategy::AlwaysMaker;
    
    for i in 0..10 {
        assert_eq!(strategy.get_order_type(i), OrderType::LimitMaker);
    }
}

#[test]
fn test_order_type_strategy_mixed() {
    let strategy = OrderTypeStrategy::Mixed { maker_percentage: 0.5 };
    
    // Test with deterministic hashing
    let mut maker_count = 0;
    let mut market_count = 0;
    
    for i in 0..1000 {
        match strategy.get_order_type(i) {
            OrderType::LimitMaker => maker_count += 1,
            _ => market_count += 1,
        }
    }
    
    // With a large enough sample, the distribution should be close to the specified percentage
    let maker_ratio = maker_count as f64 / 1000.0;
    assert!((maker_ratio - 0.5).abs() < 0.1); // Allow 10% deviation due to hash distribution
}

#[test]
fn test_order_type_strategy_mixed_extremes() {
    // Test with 0% maker
    let strategy = OrderTypeStrategy::Mixed { maker_percentage: 0.0 };
    for i in 0..10 {
        assert_eq!(strategy.get_order_type(i), OrderType::Market);
    }
    
    // Test with 100% maker
    let strategy = OrderTypeStrategy::Mixed { maker_percentage: 1.0 };
    for i in 0..10 {
        assert_eq!(strategy.get_order_type(i), OrderType::LimitMaker);
    }
}

#[test]
fn test_order_type_strategy_adaptive() {
    let strategy = OrderTypeStrategy::Adaptive;
    
    // Adaptive strategy alternates between maker and taker
    for i in 0..10 {
        if i % 2 == 0 {
            assert_eq!(strategy.get_order_type(i), OrderType::LimitMaker);
        } else {
            assert_eq!(strategy.get_order_type(i), OrderType::Market);
        }
    }
}

#[test]
fn test_commission_tracker_default() {
    let tracker = CommissionTracker::default();
    
    assert_eq!(tracker.total_maker_fees, 0.0);
    assert_eq!(tracker.total_taker_fees, 0.0);
    assert_eq!(tracker.maker_order_count, 0);
    assert_eq!(tracker.taker_order_count, 0);
}

#[test]
fn test_commission_tracker_add_commission() {
    let mut tracker = CommissionTracker::default();
    let timestamp = FixedOffset::east_opt(0).unwrap().timestamp_opt(1640995200, 0).unwrap();
    
    // Add maker commission
    tracker.add_commission(
        timestamp,
        OrderType::LimitMaker,
        10000.0,
        2.0,
        TradingScenario::OpenPosition
    );
    
    assert_eq!(tracker.total_maker_fees, 2.0);
    assert_eq!(tracker.maker_order_count, 1);
    assert_eq!(tracker.total_taker_fees, 0.0);
    assert_eq!(tracker.taker_order_count, 0);
    
    // Add market commission
    tracker.add_commission(
        timestamp,
        OrderType::Market,
        20000.0,
        10.0,
        TradingScenario::ClosePosition
    );
    
    assert_eq!(tracker.total_maker_fees, 2.0);
    assert_eq!(tracker.maker_order_count, 1);
    assert_eq!(tracker.total_taker_fees, 10.0);
    assert_eq!(tracker.taker_order_count, 1);
    
    // Add limit taker commission
    tracker.add_commission(
        timestamp,
        OrderType::LimitTaker,
        15000.0,
        7.5,
        TradingScenario::ReducePosition
    );
    
    assert_eq!(tracker.total_maker_fees, 2.0);
    assert_eq!(tracker.maker_order_count, 1);
    assert_eq!(tracker.total_taker_fees, 17.5);
    assert_eq!(tracker.taker_order_count, 2);
}

#[test]
fn test_commission_tracker_total_commission() {
    let mut tracker = CommissionTracker::default();
    let timestamp = FixedOffset::east_opt(0).unwrap().timestamp_opt(1640995200, 0).unwrap();
    
    tracker.add_commission(
        timestamp,
        OrderType::LimitMaker,
        10000.0,
        2.0,
        TradingScenario::OpenPosition
    );
    
    tracker.add_commission(
        timestamp,
        OrderType::Market,
        20000.0,
        10.0,
        TradingScenario::ClosePosition
    );
    
    assert_eq!(tracker.total_commission(), 12.0);
}

#[test]
fn test_commission_tracker_average_commission_rate() {
    let mut tracker = CommissionTracker::default();
    let timestamp = FixedOffset::east_opt(0).unwrap().timestamp_opt(1640995200, 0).unwrap();
    
    tracker.add_commission(
        timestamp,
        OrderType::LimitMaker,
        10000.0,
        2.0,
        TradingScenario::OpenPosition
    );
    
    tracker.add_commission(
        timestamp,
        OrderType::Market,
        20000.0,
        10.0,
        TradingScenario::ClosePosition
    );
    
    // Average rate = total commission / total orders = 12.0 / 2 = 6.0
    assert_eq!(tracker.average_commission_rate(), 6.0);
}

#[test]
fn test_commission_tracker_maker_taker_ratio() {
    let mut tracker = CommissionTracker::default();
    let timestamp = FixedOffset::east_opt(0).unwrap().timestamp_opt(1640995200, 0).unwrap();
    
    // Add 2 maker orders
    tracker.add_commission(
        timestamp,
        OrderType::LimitMaker,
        10000.0,
        2.0,
        TradingScenario::OpenPosition
    );
    
    tracker.add_commission(
        timestamp,
        OrderType::LimitMaker,
        10000.0,
        2.0,
        TradingScenario::OpenPosition
    );
    
    // Add 3 taker orders
    tracker.add_commission(
        timestamp,
        OrderType::Market,
        20000.0,
        10.0,
        TradingScenario::ClosePosition
    );
    
    tracker.add_commission(
        timestamp,
        OrderType::Market,
        20000.0,
        10.0,
        TradingScenario::ClosePosition
    );
    
    tracker.add_commission(
        timestamp,
        OrderType::LimitTaker,
        15000.0,
        7.5,
        TradingScenario::ReducePosition
    );
    
    // Maker/taker ratio = maker orders / total orders = 2 / 5 = 0.4
    assert_eq!(tracker.maker_taker_ratio(), 0.4);
}

#[test]
fn test_commission_tracker_empty() {
    let tracker = CommissionTracker::default();
    
    assert_eq!(tracker.total_commission(), 0.0);
    assert_eq!(tracker.average_commission_rate(), 0.0);
    assert_eq!(tracker.maker_taker_ratio(), 0.0);
}