use std::collections::HashMap;
use chrono::{DateTime, FixedOffset, Utc};

use crate::mode_reporting::{
    ModeReportingManager, CommonPerformanceMetrics, PaperTradingReport, LiveTradingReport,
    RealTimePnLReport, MonitoringDashboardData, FundingImpactAnalysis, PositionSnapshot,
    RiskMetrics, ConnectionMetrics, AlertEntry, OrderSummary
};
use crate::trading_mode::TradingMode;
use crate::unified_data::{Position, OrderSide};
use crate::paper_trading::TradeLogEntry;

// Helper function to create a test position
fn create_test_position(symbol: &str, size: f64, entry_price: f64, current_price: f64) -> Position {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let mut position = Position::new(symbol, size, entry_price, current_price, now);
    
    // Set some PnL values
    position.unrealized_pnl = size * (current_price - entry_price);
    position.realized_pnl = 100.0;
    position.funding_pnl = 25.0;
    
    position
}
//
 Helper function to create test performance metrics
fn create_test_performance_metrics(mode: TradingMode) -> CommonPerformanceMetrics {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    CommonPerformanceMetrics {
        mode,
        initial_balance: 10000.0,
        current_balance: 11000.0,
        realized_pnl: 800.0,
        unrealized_pnl: 300.0,
        funding_pnl: 50.0,
        total_pnl: 1150.0,
        total_fees: 50.0,
        total_return_pct: 11.5,
        trade_count: 10,
        win_rate: 70.0,
        max_drawdown: 500.0,
        max_drawdown_pct: 5.0,
        start_time: now - chrono::Duration::days(10),
        end_time: now,
        duration_days: 10.0,
    }
}

// Helper function to create test positions map
fn create_test_positions() -> HashMap<String, Position> {
    let mut positions = HashMap::new();
    
    positions.insert(
        "BTC".to_string(),
        create_test_position("BTC", 1.0, 50000.0, 52000.0)
    );
    
    positions.insert(
        "ETH".to_string(),
        create_test_position("ETH", -10.0, 3000.0, 2900.0)
    );
    
    positions
}

// Helper function to create test trade log
fn create_test_trade_log() -> Vec<TradeLogEntry> {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    vec![
        TradeLogEntry {
            id: "1".to_string(),
            symbol: "BTC".to_string(),
            side: OrderSide::Buy,
            quantity: 1.0,
            price: 50000.0,
            timestamp: now - chrono::Duration::days(5),
            fees: 25.0,
            order_type: crate::unified_data::OrderType::Market,
            order_id: "order1".to_string(),
            pnl: None,
            metadata: HashMap::new(),
        },
        TradeLogEntry {
            id: "2".to_string(),
            symbol: "ETH".to_string(),
            side: OrderSide::Sell,
            quantity: 10.0,
            price: 3000.0,
            timestamp: now - chrono::Duration::days(3),
            fees: 15.0,
            order_type: crate::unified_data::OrderType::Limit,
            order_id: "order2".to_string(),
            pnl: Some(100.0),
            metadata: HashMap::new(),
        },
    ]
}/
/ Helper function to create test funding impact analysis
fn create_test_funding_impact() -> FundingImpactAnalysis {
    let mut funding_by_symbol = HashMap::new();
    
    funding_by_symbol.insert(
        "BTC".to_string(),
        crate::mode_reporting::SymbolFundingMetrics {
            symbol: "BTC".to_string(),
            funding_pnl: 30.0,
            avg_funding_rate: 0.0001,
            funding_volatility: 0.00005,
            funding_received: 50.0,
            funding_paid: 20.0,
            payment_count: 15,
        }
    );
    
    funding_by_symbol.insert(
        "ETH".to_string(),
        crate::mode_reporting::SymbolFundingMetrics {
            symbol: "ETH".to_string(),
            funding_pnl: 20.0,
            avg_funding_rate: 0.0002,
            funding_volatility: 0.0001,
            funding_received: 30.0,
            funding_paid: 10.0,
            payment_count: 12,
        }
    );
    
    FundingImpactAnalysis {
        total_funding_pnl: 50.0,
        funding_pnl_percentage: 4.35,  // 50.0 / 1150.0 * 100
        avg_funding_rate: 0.00015,
        funding_rate_volatility: 0.000075,
        funding_received: 80.0,
        funding_paid: 30.0,
        payment_count: 27,
        funding_price_correlation: 0.3,
        funding_by_symbol,
    }
}

// Helper function to create test risk metrics
fn create_test_risk_metrics() -> RiskMetrics {
    RiskMetrics {
        current_leverage: 2.5,
        max_leverage: 3.0,
        value_at_risk_95: 500.0,
        value_at_risk_99: 800.0,
        expected_shortfall_95: 600.0,
        beta: 1.2,
        correlation: 0.8,
        position_concentration: 0.7,
        largest_position: 52000.0,
        largest_position_symbol: "BTC".to_string(),
    }
}

// Helper function to create test connection metrics
fn create_test_connection_metrics() -> ConnectionMetrics {
    ConnectionMetrics {
        uptime_pct: 99.8,
        disconnection_count: 2,
        avg_reconnection_time_ms: 150.0,
        api_latency_ms: 50.0,
        ws_latency_ms: 20.0,
        order_latency_ms: 80.0,
    }
}// Helpe
r function to create test alert entries
fn create_test_alerts() -> Vec<AlertEntry> {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    vec![
        AlertEntry {
            level: "WARNING".to_string(),
            message: "High volatility detected".to_string(),
            timestamp: now - chrono::Duration::hours(2),
            symbol: Some("BTC".to_string()),
            order_id: None,
        },
        AlertEntry {
            level: "ERROR".to_string(),
            message: "Order execution failed".to_string(),
            timestamp: now - chrono::Duration::hours(1),
            symbol: Some("ETH".to_string()),
            order_id: Some("order3".to_string()),
        },
    ]
}

#[test]
fn test_mode_reporting_manager_creation() {
    let manager = ModeReportingManager::new(TradingMode::PaperTrade, 10000.0);
    
    // Basic validation that the manager was created
    assert_eq!(manager.get_mode(), TradingMode::PaperTrade);
    assert_eq!(manager.get_initial_balance(), 10000.0);
}

#[test]
fn test_update_performance() {
    let mut manager = ModeReportingManager::new(TradingMode::PaperTrade, 10000.0);
    let metrics = create_test_performance_metrics(TradingMode::PaperTrade);
    
    manager.update_performance(metrics.clone());
    
    // Verify the metrics were stored
    let latest_metrics = manager.get_latest_performance_metrics().unwrap();
    assert_eq!(latest_metrics.current_balance, metrics.current_balance);
    assert_eq!(latest_metrics.realized_pnl, metrics.realized_pnl);
    assert_eq!(latest_metrics.unrealized_pnl, metrics.unrealized_pnl);
    assert_eq!(latest_metrics.funding_pnl, metrics.funding_pnl);
}#[
test]
fn test_update_positions() {
    let mut manager = ModeReportingManager::new(TradingMode::PaperTrade, 10000.0);
    let positions = create_test_positions();
    
    // Convert positions to position snapshots
    let position_snapshots = manager.convert_positions_to_snapshots(&positions);
    
    manager.update_positions(position_snapshots);
    
    // Verify positions were stored
    let latest_positions = manager.get_latest_positions().unwrap();
    assert_eq!(latest_positions.len(), positions.len());
    assert!(latest_positions.contains_key("BTC"));
    assert!(latest_positions.contains_key("ETH"));
}

#[test]
fn test_update_pnl() {
    let mut manager = ModeReportingManager::new(TradingMode::PaperTrade, 10000.0);
    let positions = create_test_positions();
    
    let pnl_report = manager.generate_real_time_pnl_report(11000.0, positions.clone()).unwrap();
    manager.update_pnl(pnl_report.clone());
    
    // Verify PnL report was stored
    let latest_pnl = manager.get_latest_pnl_report().unwrap();
    assert_eq!(latest_pnl.current_balance, pnl_report.current_balance);
    assert_eq!(latest_pnl.realized_pnl, pnl_report.realized_pnl);
    assert_eq!(latest_pnl.unrealized_pnl, pnl_report.unrealized_pnl);
    assert_eq!(latest_pnl.funding_pnl, pnl_report.funding_pnl);
}

#[test]
fn test_add_alert() {
    let mut manager = ModeReportingManager::new(TradingMode::LiveTrade, 10000.0);
    let alerts = create_test_alerts();
    
    for alert in &alerts {
        manager.add_alert(alert.clone());
    }
    
    // Verify alerts were stored
    let stored_alerts = manager.get_alerts();
    assert_eq!(stored_alerts.len(), alerts.len());
    assert_eq!(stored_alerts[0].level, alerts[0].level);
    assert_eq!(stored_alerts[0].message, alerts[0].message);
    assert_eq!(stored_alerts[1].level, alerts[1].level);
    assert_eq!(stored_alerts[1].message, alerts[1].message);
}#[t
est]
fn test_update_funding_impact() {
    let mut manager = ModeReportingManager::new(TradingMode::PaperTrade, 10000.0);
    let funding_impact = create_test_funding_impact();
    
    manager.update_funding_impact(funding_impact.clone());
    
    // Verify funding impact was stored
    let stored_impact = manager.get_funding_impact().unwrap();
    assert_eq!(stored_impact.total_funding_pnl, funding_impact.total_funding_pnl);
    assert_eq!(stored_impact.funding_pnl_percentage, funding_impact.funding_pnl_percentage);
    assert_eq!(stored_impact.avg_funding_rate, funding_impact.avg_funding_rate);
}

#[test]
fn test_update_risk_metrics() {
    let mut manager = ModeReportingManager::new(TradingMode::LiveTrade, 10000.0);
    let risk_metrics = create_test_risk_metrics();
    
    manager.update_risk_metrics(risk_metrics.clone());
    
    // Verify risk metrics were stored
    let stored_metrics = manager.get_latest_risk_metrics().unwrap();
    assert_eq!(stored_metrics.current_leverage, risk_metrics.current_leverage);
    assert_eq!(stored_metrics.max_leverage, risk_metrics.max_leverage);
    assert_eq!(stored_metrics.value_at_risk_95, risk_metrics.value_at_risk_95);
}

#[test]
fn test_update_connection_metrics() {
    let mut manager = ModeReportingManager::new(TradingMode::LiveTrade, 10000.0);
    let connection_metrics = create_test_connection_metrics();
    
    manager.update_connection_metrics(connection_metrics.clone());
    
    // Verify connection metrics were stored
    let stored_metrics = manager.get_latest_connection_metrics().unwrap();
    assert_eq!(stored_metrics.uptime_pct, connection_metrics.uptime_pct);
    assert_eq!(stored_metrics.disconnection_count, connection_metrics.disconnection_count);
    assert_eq!(stored_metrics.api_latency_ms, connection_metrics.api_latency_ms);
}#[test
]
fn test_generate_paper_trading_report() {
    let mut manager = ModeReportingManager::new(TradingMode::PaperTrade, 10000.0);
    let metrics = create_test_performance_metrics(TradingMode::PaperTrade);
    let positions = create_test_positions();
    let trade_log = create_test_trade_log();
    let funding_impact = create_test_funding_impact();
    
    manager.update_performance(metrics);
    manager.update_funding_impact(funding_impact);
    
    let report = manager.generate_paper_trading_report(trade_log, positions).unwrap();
    
    // Verify report contents
    assert_eq!(report.common.current_balance, 11000.0);
    assert_eq!(report.common.realized_pnl, 800.0);
    assert_eq!(report.common.unrealized_pnl, 300.0);
    assert_eq!(report.common.funding_pnl, 50.0);
    assert_eq!(report.funding_impact.total_funding_pnl, 50.0);
    assert_eq!(report.trade_log.len(), 2);
    assert_eq!(report.positions.len(), 2);
}

#[test]
fn test_generate_live_trading_report() {
    let mut manager = ModeReportingManager::new(TradingMode::LiveTrade, 10000.0);
    let metrics = create_test_performance_metrics(TradingMode::LiveTrade);
    let positions = create_test_positions();
    let trade_log = create_test_trade_log();
    let funding_impact = create_test_funding_impact();
    let risk_metrics = create_test_risk_metrics();
    let connection_metrics = create_test_connection_metrics();
    let alerts = create_test_alerts();
    
    manager.update_performance(metrics);
    manager.update_funding_impact(funding_impact);
    manager.update_risk_metrics(risk_metrics);
    manager.update_connection_metrics(connection_metrics);
    
    for alert in alerts {
        manager.add_alert(alert);
    }
    
    let report = manager.generate_live_trading_report(trade_log, positions).unwrap();
    
    // Verify report contents
    assert_eq!(report.common.current_balance, 11000.0);
    assert_eq!(report.common.realized_pnl, 800.0);
    assert_eq!(report.common.unrealized_pnl, 300.0);
    assert_eq!(report.common.funding_pnl, 50.0);
    assert_eq!(report.funding_impact.total_funding_pnl, 50.0);
    assert_eq!(report.risk_metrics.current_leverage, 2.5);
    assert_eq!(report.connection_metrics.uptime_pct, 99.8);
    assert_eq!(report.alert_history.len(), 2);
    assert_eq!(report.trade_log.len(), 2);
    assert_eq!(report.positions.len(), 2);
}#[te
st]
fn test_generate_real_time_pnl_report() {
    let mut manager = ModeReportingManager::new(TradingMode::PaperTrade, 10000.0);
    let positions = create_test_positions();
    
    let report = manager.generate_real_time_pnl_report(11000.0, positions).unwrap();
    
    // Verify report contents
    assert_eq!(report.current_balance, 11000.0);
    assert_eq!(report.mode, TradingMode::PaperTrade);
    assert!(report.positions.contains_key("BTC"));
    assert!(report.positions.contains_key("ETH"));
}

#[test]
fn test_generate_monitoring_dashboard() {
    let mut manager = ModeReportingManager::new(TradingMode::LiveTrade, 10000.0);
    let positions = create_test_positions();
    let order_summary = OrderSummary {
        active_orders: 5,
        filled_today: 10,
        cancelled_today: 2,
        rejected_today: 1,
        success_rate: 90.0,
        avg_fill_time_ms: 120.0,
        volume_today: 150000.0,
        fees_today: 75.0,
    };
    
    let dashboard = manager.generate_monitoring_dashboard(
        11000.0,
        9000.0,
        positions,
        5,
        order_summary
    ).unwrap();
    
    // Verify dashboard contents
    assert_eq!(dashboard.account_summary.total_equity, 11000.0 + 2000.0 + 1000.0); // balance + unrealized PnL
    assert_eq!(dashboard.account_summary.available_balance, 9000.0);
    assert_eq!(dashboard.position_summary.open_positions, 2);
    assert_eq!(dashboard.order_summary.active_orders, 5);
    assert_eq!(dashboard.order_summary.filled_today, 10);
}#[test]
fn 
test_funding_impact_analysis() {
    let mut manager = ModeReportingManager::new(TradingMode::Backtest, 10000.0);
    let positions = create_test_positions();
    
    // Create funding data
    let mut funding_rates = HashMap::new();
    funding_rates.insert("BTC".to_string(), vec![0.0001, 0.00015, 0.0002]);
    funding_rates.insert("ETH".to_string(), vec![-0.0001, -0.00015, -0.0002]);
    
    let mut funding_timestamps = HashMap::new();
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    funding_timestamps.insert("BTC".to_string(), vec![
        now - chrono::Duration::hours(16),
        now - chrono::Duration::hours(8),
        now,
    ]);
    funding_timestamps.insert("ETH".to_string(), vec![
        now - chrono::Duration::hours(16),
        now - chrono::Duration::hours(8),
        now,
    ]);
    
    let funding_impact = manager.analyze_funding_impact(&positions, &funding_rates, &funding_timestamps).unwrap();
    
    // Verify funding impact analysis
    assert!(funding_impact.total_funding_pnl != 0.0);
    assert!(funding_impact.funding_by_symbol.contains_key("BTC"));
    assert!(funding_impact.funding_by_symbol.contains_key("ETH"));
    assert!(funding_impact.avg_funding_rate != 0.0);
}

#[test]
fn test_calculate_risk_adjusted_returns() {
    let mut manager = ModeReportingManager::new(TradingMode::PaperTrade, 10000.0);
    
    // Add daily returns
    let daily_returns = vec![0.01, -0.005, 0.02, 0.015, -0.01, 0.005, 0.01];
    for return_value in daily_returns {
        manager.add_daily_return(return_value);
    }
    
    let (sharpe, sortino, volatility) = manager.calculate_risk_adjusted_returns();
    
    // Verify calculations
    assert!(sharpe > 0.0);
    assert!(sortino > 0.0);
    assert!(volatility > 0.0);
}

#[test]
fn test_calculate_period_pnl() {
    let mut manager = ModeReportingManager::new(TradingMode::PaperTrade, 10000.0);
    
    // Add PnL history
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    let pnl_report1 = RealTimePnLReport {
        timestamp: now - chrono::Duration::hours(25),
        mode: TradingMode::PaperTrade,
        current_balance: 10100.0,
        realized_pnl: 100.0,
        unrealized_pnl: 0.0,
        funding_pnl: 0.0,
        total_pnl: 100.0,
        total_return_pct: 1.0,
        positions: HashMap::new(),
        daily_pnl: 100.0,
        hourly_pnl: 20.0,
    };
    
    let pnl_report2 = RealTimePnLReport {
        timestamp: now - chrono::Duration::hours(12),
        mode: TradingMode::PaperTrade,
        current_balance: 10200.0,
        realized_pnl: 200.0,
        unrealized_pnl: 0.0,
        funding_pnl: 0.0,
        total_pnl: 200.0,
        total_return_pct: 2.0,
        positions: HashMap::new(),
        daily_pnl: 100.0,
        hourly_pnl: 10.0,
    };
    
    let pnl_report3 = RealTimePnLReport {
        timestamp: now,
        mode: TradingMode::PaperTrade,
        current_balance: 10300.0,
        realized_pnl: 300.0,
        unrealized_pnl: 0.0,
        funding_pnl: 0.0,
        total_pnl: 300.0,
        total_return_pct: 3.0,
        positions: HashMap::new(),
        daily_pnl: 100.0,
        hourly_pnl: 10.0,
    };
    
    manager.update_pnl(pnl_report1);
    manager.update_pnl(pnl_report2);
    manager.update_pnl(pnl_report3);
    
    // Calculate period PnL
    let daily_pnl = manager.calculate_period_pnl(24);
    let hourly_pnl = manager.calculate_period_pnl(1);
    
    // Verify calculations
    assert_eq!(daily_pnl, 200.0); // 300 - 100 = 200 (last 24 hours)
    assert!(hourly_pnl > 0.0);
}