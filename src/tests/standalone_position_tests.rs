use std::collections::HashMap;
use chrono::{DateTime, FixedOffset, TimeZone, Utc};

use crate::unified_data::Position;

#[test]
fn test_position_basic_functionality() {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Create a position
    let mut position = Position::new("BTC", 1.0, 50000.0, 51000.0, now);
    
    // Test initial state
    assert_eq!(position.symbol, "BTC");
    assert_eq!(position.size, 1.0);
    assert_eq!(position.entry_price, 50000.0);
    assert_eq!(position.current_price, 51000.0);
    assert_eq!(position.leverage, 1.0);
    assert_eq!(position.liquidation_price, None);
    assert_eq!(position.margin, None);
    assert!(position.is_long());
    assert!(!position.is_short());
    assert!(!position.is_flat());
    assert_eq!(position.unrealized_pnl, 1000.0); // (51000 - 50000) * 1.0
    
    // Update position price
    position.update_price(52000.0);
    assert_eq!(position.current_price, 52000.0);
    assert_eq!(position.unrealized_pnl, 2000.0); // (52000 - 50000) * 1.0
    
    // Apply funding payment
    position.apply_funding_payment(100.0);
    assert_eq!(position.funding_pnl, 100.0);
    assert_eq!(position.total_pnl(), 2100.0); // 2000 + 0 + 100
}

#[test]
fn test_position_direction_methods() {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Test long position
    let long_position = Position::new("BTC", 1.0, 50000.0, 51000.0, now);
    assert!(long_position.is_long());
    assert!(!long_position.is_short());
    assert!(!long_position.is_flat());
    
    // Test short position
    let short_position = Position::new("ETH", -2.0, 3000.0, 2900.0, now);
    assert!(!short_position.is_long());
    assert!(short_position.is_short());
    assert!(!short_position.is_flat());
    
    // Test flat position
    let flat_position = Position::new("XRP", 0.0, 1.0, 1.0, now);
    assert!(!flat_position.is_long());
    assert!(!flat_position.is_short());
    assert!(flat_position.is_flat());
}

#[test]
fn test_position_pnl_calculations() {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Create a position
    let mut position = Position::new("BTC", 1.0, 50000.0, 51000.0, now);
    assert_eq!(position.unrealized_pnl, 1000.0);
    assert_eq!(position.realized_pnl, 0.0);
    assert_eq!(position.funding_pnl, 0.0);
    assert_eq!(position.total_pnl(), 1000.0);
    assert_eq!(position.notional_value(), 51000.0); // 1.0 * 51000.0
    
    // Update price
    position.update_price(52000.0);
    assert_eq!(position.unrealized_pnl, 2000.0);
    assert_eq!(position.total_pnl(), 2000.0);
    assert_eq!(position.notional_value(), 52000.0); // 1.0 * 52000.0
    
    // Apply funding payment
    position.apply_funding_payment(100.0);
    assert_eq!(position.funding_pnl, 100.0);
    assert_eq!(position.total_pnl(), 2100.0);
    
    // Apply another funding payment
    position.apply_funding_payment(-50.0);
    assert_eq!(position.funding_pnl, 50.0);
    assert_eq!(position.total_pnl(), 2050.0);
    
    // Test short position PnL
    let mut short_position = Position::new("ETH", -2.0, 3000.0, 2900.0, now);
    assert_eq!(short_position.unrealized_pnl, 200.0); // (2900 - 3000) * -2.0
    assert_eq!(short_position.notional_value(), 5800.0); // 2.0 * 2900.0
    
    short_position.update_price(2800.0);
    assert_eq!(short_position.unrealized_pnl, 400.0); // (2800 - 3000) * -2.0
    assert_eq!(short_position.notional_value(), 5600.0); // 2.0 * 2800.0
    
    short_position.apply_funding_payment(-75.0);
    assert_eq!(short_position.funding_pnl, -75.0);
    assert_eq!(short_position.total_pnl(), 325.0); // 400 + 0 + (-75)
}

#[test]
fn test_position_with_metadata() {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Create a position with metadata
    let mut position = Position::new("BTC", 1.0, 50000.0, 51000.0, now);
    
    // Add metadata
    position.metadata.insert("strategy".to_string(), "sma_cross".to_string());
    position.metadata.insert("entry_reason".to_string(), "momentum".to_string());
    
    // Check metadata
    assert_eq!(position.metadata.get("strategy"), Some(&"sma_cross".to_string()));
    assert_eq!(position.metadata.get("entry_reason"), Some(&"momentum".to_string()));
    assert_eq!(position.metadata.get("nonexistent"), None);
    
    // Update metadata
    position.metadata.insert("entry_reason".to_string(), "breakout".to_string());
    assert_eq!(position.metadata.get("entry_reason"), Some(&"breakout".to_string()));
    
    // Set leverage directly
    position.leverage = 5.0;
    assert_eq!(position.leverage, 5.0);
}

#[test]
fn test_position_with_zero_size() {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Create a position with zero size
    let mut position = Position::new("BTC", 0.0, 50000.0, 51000.0, now);
    
    // Test properties
    assert_eq!(position.size, 0.0);
    assert_eq!(position.unrealized_pnl, 0.0);
    assert_eq!(position.notional_value(), 0.0);
    assert!(position.is_flat());
    
    // Update price should not affect PnL
    position.update_price(52000.0);
    assert_eq!(position.unrealized_pnl, 0.0);
    
    // Funding payments still apply
    position.apply_funding_payment(100.0);
    assert_eq!(position.funding_pnl, 100.0);
    assert_eq!(position.total_pnl(), 100.0);
}

#[test]
fn test_position_with_leverage_and_liquidation() {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Create a position with leverage and liquidation price
    let mut position = Position::new("BTC", 1.0, 50000.0, 51000.0, now);
    position.leverage = 5.0;
    position.liquidation_price = Some(45000.0);
    position.margin = Some(10000.0);
    
    // Test properties
    assert_eq!(position.leverage, 5.0);
    assert_eq!(position.liquidation_price, Some(45000.0));
    assert_eq!(position.margin, Some(10000.0));
    
    // Update price
    position.update_price(48000.0);
    assert_eq!(position.unrealized_pnl, -2000.0); // (48000 - 50000) * 1.0
    
    // Still above liquidation price
    assert!(position.current_price > position.liquidation_price.unwrap());
    
    // Update price below liquidation
    position.update_price(44000.0);
    assert_eq!(position.unrealized_pnl, -6000.0); // (44000 - 50000) * 1.0
    
    // Now below liquidation price
    assert!(position.current_price < position.liquidation_price.unwrap());
}