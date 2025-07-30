use std::time::Duration;
use chrono::{Utc, FixedOffset};
use tokio::time::sleep;

use hyperliquid_backtest::prelude::*;
use hyperliquid_backtest::real_time_monitoring::{
    MonitoringServer, MonitoringClient, MonitoringManager,
    MonitoringMessage, TradeExecutionUpdate, ConnectionStatus
};
use hyperliquid_backtest::live_trading::{LiveTradingEngine, AlertLevel};
use hyperliquid_backtest::unified_data::{OrderRequest, OrderResult, OrderSide, OrderType, OrderStatus, TimeInForce};
use hyperliquid_backtest::trading_mode::TradingMode;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    init_logger();
    
    println!("Starting real-time monitoring example...");
    
    // Create monitoring server
    let port = 8080;
    let mut server = MonitoringServer::new(port);
    server.start().await.expect("Failed to start monitoring server");
    
    println!("Monitoring server started on port {}", port);
    
    // Create monitoring manager
    let mut manager = MonitoringManager::new(TradingMode::LiveTrade);
    
    // Add alert handler
    manager.add_alert_handler(|alert| {
        println!("Alert received: {} - {}", alert.level, alert.message);
    });
    
    // Add trade execution handler
    manager.add_trade_execution_handler(|execution| {
        println!("Trade execution: {} - {} - {:?}", execution.order_id, execution.symbol, execution.status);
    });
    
    // Send some alerts
    println!("Sending alerts...");
    manager.send_alert(AlertLevel::Info, "System started", None, None)?;
    manager.send_alert(AlertLevel::Warning, "High volatility detected", Some("BTC"), None)?;
    
    // Record some trade executions
    println!("Recording trade executions...");
    
    let order_request = OrderRequest {
        symbol: "BTC".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Market,
        quantity: 1.0,
        price: None,
        reduce_only: false,
        time_in_force: TimeInForce::GoodTilCancelled,
    };
    
    let order_result = OrderResult {
        order_id: "order1".to_string(),
        status: OrderStatus::Filled,
        filled_quantity: 1.0,
        average_price: Some(50000.0),
        fees: Some(25.0),
        timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
        error: None,
    };
    
    manager.record_trade_execution(&order_request, &order_result, 120)?;
    
    // Update performance metrics
    println!("Updating performance metrics...");
    manager.update_performance_metrics(
        10000.0, // current_balance
        100.0,   // daily_pnl
        500.0,   // total_pnl
        0.6,     // win_rate
        1.5,     // sharpe_ratio
        5.0,     // max_drawdown_pct
        2        // positions_count
    )?;
    
    // Update connection metrics
    println!("Updating connection metrics...");
    manager.update_connection_metrics(
        99.5,    // uptime_pct
        2,       // disconnection_count
        150.0,   // avg_reconnection_time_ms
        50.0,    // api_latency_ms
        25.0     // ws_latency_ms
    )?;
    
    // Create dashboard data
    println!("Updating dashboard...");
    let dashboard_data = create_sample_dashboard_data();
    manager.update_dashboard(dashboard_data)?;
    
    // Wait for a while to allow clients to connect
    println!("Server running. Press Ctrl+C to stop...");
    
    // In a real application, we would keep the server running
    // For this example, we'll just sleep for a while
    sleep(Duration::from_secs(60)).await;
    
    // Stop server
    println!("Stopping server...");
    server.stop().await?;
    
    println!("Example completed successfully!");
    Ok(())
}

fn create_sample_dashboard_data() -> hyperliquid_backtest::mode_reporting::MonitoringDashboardData {
    use hyperliquid_backtest::mode_reporting::*;
    use std::collections::HashMap;
    
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Create sample risk allocation
    let mut risk_allocation = HashMap::new();
    risk_allocation.insert("BTC".to_string(), 60.0);
    risk_allocation.insert("ETH".to_string(), 40.0);
    
    // Create sample alerts
    let mut recent_alerts = Vec::new();
    recent_alerts.push(AlertEntry {
        level: "Info".to_string(),
        message: "System started".to_string(),
        timestamp: now - chrono::Duration::minutes(5),
        symbol: None,
        order_id: None,
    });
    recent_alerts.push(AlertEntry {
        level: "Warning".to_string(),
        message: "High volatility detected".to_string(),
        timestamp: now - chrono::Duration::minutes(2),
        symbol: Some("BTC".to_string()),
        order_id: None,
    });
    
    MonitoringDashboardData {
        timestamp: now,
        account_summary: AccountSummary {
            total_equity: 10000.0,
            available_balance: 9000.0,
            margin_used: 1000.0,
            margin_usage_pct: 10.0,
            current_leverage: 1.0,
            unrealized_pnl: 100.0,
            realized_pnl: 200.0,
            funding_pnl: 50.0,
        },
        position_summary: PositionSummary {
            open_positions: 2,
            long_positions: 1,
            short_positions: 1,
            total_position_value: 10000.0,
            largest_position: PositionSnapshot {
                symbol: "BTC".to_string(),
                size: 1.0,
                entry_price: 50000.0,
                current_price: 51000.0,
                unrealized_pnl: 1000.0,
                unrealized_pnl_pct: 2.0,
                funding_pnl: 50.0,
                liquidation_price: None,
                side: OrderSide::Buy,
                position_age_hours: 24.0,
            },
            most_profitable: PositionSnapshot {
                symbol: "BTC".to_string(),
                size: 1.0,
                entry_price: 50000.0,
                current_price: 51000.0,
                unrealized_pnl: 1000.0,
                unrealized_pnl_pct: 2.0,
                funding_pnl: 50.0,
                liquidation_price: None,
                side: OrderSide::Buy,
                position_age_hours: 24.0,
            },
            least_profitable: PositionSnapshot {
                symbol: "ETH".to_string(),
                size: -2.0,
                entry_price: 3000.0,
                current_price: 3050.0,
                unrealized_pnl: -100.0,
                unrealized_pnl_pct: -1.67,
                funding_pnl: -10.0,
                liquidation_price: None,
                side: OrderSide::Sell,
                position_age_hours: 12.0,
            },
        },
        order_summary: OrderSummary {
            active_orders: 1,
            filled_today: 5,
            cancelled_today: 2,
            rejected_today: 0,
            success_rate: 0.8,
            avg_fill_time_ms: 120.0,
            volume_today: 50000.0,
            fees_today: 25.0,
        },
        risk_summary: RiskSummary {
            current_drawdown_pct: 2.0,
            max_drawdown_pct: 5.0,
            value_at_risk: 500.0,
            daily_volatility: 1.5,
            risk_allocation,
            risk_warnings: Vec::new(),
            circuit_breaker_status: "Normal".to_string(),
        },
        system_status: SystemStatus {
            connection_status: "Connected".to_string(),
            api_latency_ms: 50.0,
            ws_latency_ms: 25.0,
            uptime_hours: 24.0,
            memory_usage_mb: 100.0,
            cpu_usage_pct: 5.0,
            last_error: None,
            last_error_time: None,
        },
        recent_alerts,
        performance: PerformanceSnapshot {
            daily_pnl: 100.0,
            daily_pnl_pct: 1.0,
            weekly_pnl: 500.0,
            weekly_pnl_pct: 5.0,
            monthly_pnl: 2000.0,
            monthly_pnl_pct: 20.0,
            sharpe_ratio: 1.5,
            sortino_ratio: 2.0,
            win_rate: 0.6,
            avg_win: 200.0,
            avg_loss: -100.0,
            profit_factor: 2.0,
        },
    }
}