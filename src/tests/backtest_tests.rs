//! Tests for the backtest module

use crate::backtest::*;
use crate::data::HyperliquidData;
use crate::errors::Result;
use chrono::{DateTime, FixedOffset, TimeZone};
use rs_backtester::strategies::Strategy;

// Helper function to create test data
fn create_test_data() -> HyperliquidData {
    let mut datetime = Vec::new();
    let mut open = Vec::new();
    let mut high = Vec::new();
    let mut low = Vec::new();
    let mut close = Vec::new();
    let mut volume = Vec::new();
    let mut funding_rates = Vec::new();
    
    // Create 10 days of hourly data
    let base_timestamp = 1640995200; // 2022-01-01 00:00:00 UTC
    
    for i in 0..10*24 {
        let timestamp = FixedOffset::east_opt(0).unwrap()
            .timestamp_opt(base_timestamp + i * 3600, 0).unwrap();
        
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
        
        // Add funding rates every 8 hours (0:00, 8:00, 16:00)
        if timestamp.hour() % 8 == 0 {
            // Create funding rate pattern
            let funding_cycle = ((i as f64) * 0.05).sin() * 0.0002;
            funding_rates.push(funding_cycle);
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

// Helper function to create a simple strategy
fn create_test_strategy(data: rs_backtester::datas::Data) -> Strategy {
    // Simple strategy that goes long when price is above 100, short when below
    let mut strategy = Strategy::new();
    
    strategy.next(Box::new(move |ctx, _| {
        let index = ctx.index();
        if index < 5 {
            return; // Skip first few candles
        }
        
        let price = ctx.data().close[index];
        
        if price > 100.0 {
            ctx.entry_qty(1.0);
        } else if price < 100.0 {
            ctx.entry_qty(-1.0);
        } else {
            ctx.exit();
        }
    }));
    
    strategy
}

#[test]
fn test_hyperliquid_backtest_new() {
    let data = create_test_data();
    let strategy_name = "Test Strategy".to_string();
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    );
    
    // Verify backtest initialization
    assert_eq!(backtest.strategy_name(), &strategy_name);
    assert_eq!(backtest.initial_capital(), initial_capital);
    assert_eq!(backtest.data().symbol, data.symbol);
    assert_eq!(backtest.data().len(), data.len());
    assert!(!backtest.is_initialized()); // Base backtest not initialized yet
}

#[test]
fn test_hyperliquid_backtest_with_order_type_strategy() {
    let data = create_test_data();
    let strategy_name = "Test Strategy".to_string();
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    ).with_order_type_strategy(OrderTypeStrategy::AlwaysMaker);
    
    // Verify order type strategy was set
    match backtest.order_type_strategy {
        OrderTypeStrategy::AlwaysMaker => {}, // Expected
        _ => panic!("Order type strategy not set correctly"),
    }
}

#[test]
fn test_initialize_base_backtest() -> Result<()> {
    let data = create_test_data();
    let strategy_name = "Test Strategy".to_string();
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    );
    
    // Initialize base backtest
    backtest.initialize_base_backtest()?;
    
    // Verify base backtest was initialized
    assert!(backtest.is_initialized());
    assert!(backtest.base_backtest().is_some());
    
    // Verify PnL tracking vectors were initialized
    assert_eq!(backtest.funding_pnl().len(), data.len());
    assert_eq!(backtest.trading_pnl().len(), data.len());
    
    Ok(())
}

#[test]
fn test_calculate_with_funding() -> Result<()> {
    let data = create_test_data();
    let rs_data = data.to_rs_backtester_data();
    let strategy = create_test_strategy(rs_data);
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        "Test Strategy".to_string(),
        initial_capital,
        commission.clone(),
    );
    
    // Initialize base backtest with strategy
    backtest.base_backtest = Some(rs_backtester::backtester::Backtest::new(
        rs_data,
        strategy,
        initial_capital,
        commission.to_rs_backtester_commission(),
    ));
    
    // Calculate with funding
    backtest.calculate_with_funding()?;
    
    // Verify funding calculations
    assert!(!backtest.funding_payments.is_empty());
    assert_eq!(backtest.funding_pnl.len(), data.len());
    
    // Verify enhanced metrics were updated
    assert!(backtest.enhanced_metrics.funding_payments_received >= 0);
    assert!(backtest.enhanced_metrics.funding_payments_paid >= 0);
    
    Ok(())
}

#[test]
fn test_calculate_with_funding_and_positions() -> Result<()> {
    let data = create_test_data();
    let rs_data = data.to_rs_backtester_data();
    let strategy = create_test_strategy(rs_data);
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        "Test Strategy".to_string(),
        initial_capital,
        commission.clone(),
    );
    
    // Initialize base backtest with strategy
    backtest.base_backtest = Some(rs_backtester::backtester::Backtest::new(
        rs_data,
        strategy,
        initial_capital,
        commission.to_rs_backtester_commission(),
    ));
    
    // Create position array (alternating between long and short)
    let positions: Vec<f64> = (0..data.len())
        .map(|i| if i % 16 < 8 { 1.0 } else { -1.0 })
        .collect();
    
    // Calculate with funding and positions
    backtest.calculate_with_funding_and_positions(&positions)?;
    
    // Verify funding calculations
    assert!(!backtest.funding_payments.is_empty());
    assert_eq!(backtest.funding_pnl.len(), data.len());
    
    // Verify enhanced metrics were updated
    assert!(backtest.enhanced_metrics.funding_payments_received >= 0);
    assert!(backtest.enhanced_metrics.funding_payments_paid >= 0);
    
    Ok(())
}

#[test]
fn test_validate() -> Result<()> {
    let data = create_test_data();
    let strategy_name = "Test Strategy".to_string();
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    );
    
    // Validate should pass with valid parameters
    let result = backtest.validate();
    assert!(result.is_ok());
    
    // Test with invalid initial capital
    let backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        0.0, // Invalid initial capital
        commission.clone(),
    );
    
    let result = backtest.validate();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Initial capital must be positive"));
    }
    
    // Test with empty strategy name
    let backtest = HyperliquidBacktest::new(
        data.clone(),
        "".to_string(), // Empty strategy name
        initial_capital,
        commission.clone(),
    );
    
    let result = backtest.validate();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Strategy name cannot be empty"));
    }
    
    Ok(())
}

#[test]
fn test_commission_stats() -> Result<()> {
    let data = create_test_data();
    let strategy_name = "Test Strategy".to_string();
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    );
    
    // Add some commission entries
    let timestamp = FixedOffset::east_opt(0).unwrap()
        .timestamp_opt(1640995200, 0).unwrap();
    
    backtest.track_commission(
        timestamp,
        OrderType::LimitMaker,
        10000.0,
        2.0,
        TradingScenario::OpenPosition
    );
    
    backtest.track_commission(
        timestamp,
        OrderType::Market,
        20000.0,
        10.0,
        TradingScenario::ClosePosition
    );
    
    // Get commission stats
    let stats = backtest.commission_stats();
    
    // Verify stats
    assert_eq!(stats.total_commission, 12.0);
    assert_eq!(stats.maker_fees, 2.0);
    assert_eq!(stats.taker_fees, 10.0);
    assert_eq!(stats.maker_orders, 1);
    assert_eq!(stats.taker_orders, 1);
    assert_eq!(stats.average_rate, 6.0); // (12.0 / 2)
    assert_eq!(stats.maker_taker_ratio, 0.5); // (1 / 2)
    
    Ok(())
}

#[test]
fn test_calculate_trade_commission() {
    let data = create_test_data();
    let strategy_name = "Test Strategy".to_string();
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    // Test with AlwaysMarket strategy
    let backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    ).with_order_type_strategy(OrderTypeStrategy::AlwaysMarket);
    
    let (order_type, fee) = backtest.calculate_trade_commission(
        10000.0,
        0,
        TradingScenario::OpenPosition
    );
    
    assert_eq!(order_type, OrderType::Market);
    assert_eq!(fee, 10000.0 * 0.0005); // Taker fee
    
    // Test with AlwaysMaker strategy
    let backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    ).with_order_type_strategy(OrderTypeStrategy::AlwaysMaker);
    
    let (order_type, fee) = backtest.calculate_trade_commission(
        10000.0,
        0,
        TradingScenario::OpenPosition
    );
    
    assert_eq!(order_type, OrderType::LimitMaker);
    assert_eq!(fee, 10000.0 * 0.0002); // Maker fee
    
    // Test with Mixed strategy
    let backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    ).with_order_type_strategy(OrderTypeStrategy::Mixed { maker_percentage: 0.0 });
    
    let (order_type, fee) = backtest.calculate_trade_commission(
        10000.0,
        0,
        TradingScenario::OpenPosition
    );
    
    assert_eq!(order_type, OrderType::Market);
    assert_eq!(fee, 10000.0 * 0.0005); // Taker fee
}

#[test]
fn test_funding_summary() -> Result<()> {
    let data = create_test_data();
    let strategy_name = "Test Strategy".to_string();
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    );
    
    // Initialize base backtest
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

#[test]
fn test_enhanced_report() -> Result<()> {
    let data = create_test_data();
    let rs_data = data.to_rs_backtester_data();
    let strategy = create_test_strategy(rs_data);
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        "Test Strategy".to_string(),
        initial_capital,
        commission.clone(),
    );
    
    // Initialize base backtest with strategy
    backtest.base_backtest = Some(rs_backtester::backtester::Backtest::new(
        rs_data,
        strategy,
        initial_capital,
        commission.to_rs_backtester_commission(),
    ));
    
    // Calculate with funding
    backtest.calculate_with_funding()?;
    
    // Get enhanced report
    let report = backtest.enhanced_report();
    
    // Verify report fields
    assert_eq!(report.strategy_name, "Test Strategy");
    assert_eq!(report.ticker, "BTC");
    assert_eq!(report.initial_capital, initial_capital);
    
    // Verify enhanced metrics
    assert_eq!(report.enhanced_metrics.funding_only_return, backtest.enhanced_metrics.funding_only_return);
    assert_eq!(report.enhanced_metrics.trading_only_return, backtest.enhanced_metrics.trading_only_return);
    assert_eq!(report.enhanced_metrics.total_return_with_funding, backtest.enhanced_metrics.total_return_with_funding);
    
    // Verify commission stats
    assert_eq!(report.commission_stats.total_commission, backtest.commission_tracker.total_commission());
    assert_eq!(report.commission_stats.maker_fees, backtest.commission_tracker.total_maker_fees);
    assert_eq!(report.commission_stats.taker_fees, backtest.commission_tracker.total_taker_fees);
    
    // Verify funding summary
    assert_eq!(report.funding_summary.total_funding_paid, backtest.total_funding_paid);
    assert_eq!(report.funding_summary.total_funding_received, backtest.total_funding_received);
    assert_eq!(report.funding_summary.net_funding, backtest.total_funding_received - backtest.total_funding_paid);
    
    Ok(())
}