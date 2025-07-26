use std::time::Duration;
use std::sync::{Arc, Mutex};
use chrono::{Utc, FixedOffset};
use tokio::time::sleep;

use hyperliquid_backtest::prelude::*;
use hyperliquid_backtest::real_time_monitoring::{
    MonitoringServer, MonitoringClient, MonitoringManager,
    MonitoringMessage, TradeExecutionUpdate, ConnectionStatus,
    PerformanceMetricsUpdate, ConnectionStatusUpdate
};
use hyperliquid_backtest::mode_reporting::{
    MonitoringDashboardData, RealTimePnLReport, AlertEntry,
    ConnectionMetrics, RiskMetrics, PositionSnapshot, AccountSummary,
    PositionSummary, OrderSummary, RiskSummary, SystemStatus, PerformanceSnapshot
};
use hyperliquid_backtest::live_trading::{LiveTradingEngine, AlertLevel};
use hyperliquid_backtest::unified_data::{OrderRequest, OrderResult, OrderSide, OrderType, OrderStatus, TimeInForce};
use hyperliquid_backtest::trading_mode::TradingMode;

/// # Real-Time Monitoring and Alerting Setup Example
///
/// This example demonstrates how to set up comprehensive real-time monitoring and alerting
/// for trading systems on Hyperliquid, including:
///
/// - Setting up a monitoring server and client architecture
/// - Configuring real-time performance metrics streaming
/// - Implementing multi-level alerting system
/// - Creating custom alert handlers for different alert types
/// - Setting up real-time dashboard with key trading metrics
/// - Implementing trade execution monitoring
/// - Creating connection status monitoring
/// - Setting up emergency notification systems
/// - Implementing custom monitoring dashboards

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Real-Time Monitoring and Alerting Setup Example");
    println!("==============================================\n");

    // 1. Set up monitoring server
    println!("1. Setting up monitoring server...");
    let port = 8080;
    let mut server = MonitoringServer::new(port);
    server.start().await?;
    println!("  Monitoring server started on port {}", port);
    
    // 2. Create monitoring manager for live trading
    println!("\n2. Creating monitoring manager for live trading...");
    let mut manager = MonitoringManager::new(TradingMode::LiveTrade);
    
    // 3. Configure alert handlers
    println!("\n3. Configuring alert handlers...");
    
    // Add console alert handler
    manager.add_alert_handler(|alert| {
        let prefix = match alert.level.as_str() {
            "Critical" => "🚨 CRITICAL",
            "Error" => "❌ ERROR",
            "Warning" => "⚠️ WARNING",
            "Info" => "ℹ️ INFO",
            _ => "ALERT",
        };
        println!("  {} - {} - {}", prefix, alert.timestamp, alert.message);
    });
    
    // Add trade execution handler
    manager.add_trade_execution_handler(|execution| {
        let status_symbol = match execution.status {
            OrderStatus::Filled => "✅",
            OrderStatus::PartiallyFilled => "⚠️",
            OrderStatus::Rejected => "❌",
            OrderStatus::Cancelled => "🚫",
            _ => "⏳",
        };
        println!("  {} Trade execution: {} - {} - {:?} - ${:.2}", 
                 status_symbol, execution.order_id, execution.symbol, 
                 execution.status, execution.average_price.unwrap_or(0.0));
    });
    
    // 4. Set up monitoring client
    println!("\n4. Setting up monitoring client...");
    let mut client = MonitoringClient::new("ws://localhost:8080");
    
    // Add message handlers to client
    let alert_count = Arc::new(Mutex::new(0));
    let alert_count_clone = alert_count.clone();
    
    client.add_message_handler(move |message| {
        match message {
            MonitoringMessage::Alert(alert) => {
                println!("  Client received alert: {} - {}", alert.level, alert.message);
                let mut count = alert_count_clone.lock().unwrap();
                *count += 1;
            },
            MonitoringMessage::Dashboard(_) => {
                println!("  Client received dashboard update");
            },
            MonitoringMessage::PnL(pnl) => {
                println!("  Client received PnL update: ${:.2}", pnl.current_pnl);
            },
            MonitoringMessage::TradeExecution(exec) => {
                println!("  Client received trade execution: {} - {}", exec.order_id, exec.symbol);
            },
            MonitoringMessage::PerformanceMetrics(metrics) => {
                println!("  Client received performance metrics: ${:.2} balance", metrics.current_balance);
            },
            MonitoringMessage::ConnectionStatus(status) => {
                println!("  Client received connection status: {:?}", status.status);
            },
            _ => {}
        }
    });
    
    // Connect client to server
    client.connect().await?;
    println!("  Monitoring client connected");
    
    // 5. Send different types of alerts
    println!("\n5. Sending different types of alerts...");
    
    // Info alert
    manager.send_alert(AlertLevel::Info, "System started successfully", None, None)?;
    
    // Warning alert
    manager.send_alert(AlertLevel::Warning, "High volatility detected on BTC", Some("BTC"), None)?;
    
    // Error alert
    manager.send_alert(AlertLevel::Error, "API rate limit exceeded", None, None)?;
    
    // Critical alert
    manager.send_alert(AlertLevel::Critical, "Margin call imminent on ETH position", Some("ETH"), Some("order123"))?;
    
    // 6. Record trade executions
    println!("\n6. Recording trade executions...");
    
    // Create sample orders and results
    let orders = vec![
        (
            OrderRequest {
                symbol: "BTC".to_string(),
                side: OrderSide::Buy,
                order_type: OrderType::Market,
                quantity: 1.0,
                price: None,
                reduce_only: false,
                time_in_force: TimeInForce::GoodTilCancelled,
            },
            OrderResult {
                order_id: "order1".to_string(),
                status: OrderStatus::Filled,
                filled_quantity: 1.0,
                average_price: Some(50000.0),
                fees: Some(25.0),
                timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
                error: None,
            },
            120, // 120ms latency
        ),
        (
            OrderRequest {
                symbol: "ETH".to_string(),
                side: OrderSide::Buy,
                order_type: OrderType::Limit,
                quantity: 10.0,
                price: Some(3000.0),
                reduce_only: false,
                time_in_force: TimeInForce::GoodTilCancelled,
            },
            OrderResult {
                order_id: "order2".to_string(),
                status: OrderStatus::PartiallyFilled,
                filled_quantity: 5.0,
                average_price: Some(3000.0),
                fees: Some(7.5),
                timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
                error: None,
            },
            95, // 95ms latency
        ),
        (
            OrderRequest {
                symbol: "SOL".to_string(),
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                quantity: 100.0,
                price: Some(100.0),
                reduce_only: true,
                time_in_force: TimeInForce::GoodTilCancelled,
            },
            OrderResult {
                order_id: "order3".to_string(),
                status: OrderStatus::Rejected,
                filled_quantity: 0.0,
                average_price: None,
                fees: None,
                timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
                error: Some("Insufficient balance".to_string()),
            },
            50, // 50ms latency
        ),
    ];
    
    for (request, result, latency) in orders {
        manager.record_trade_execution(&request, &result, latency)?;
    }
    
    // 7. Update performance metrics
    println!("\n7. Updating performance metrics...");
    
    manager.update_performance_metrics(
        100000.0, // current_balance
        1500.0,   // daily_pnl
        5000.0,   // total_pnl
        0.65,     // win_rate
        1.8,      // sharpe_ratio
        7.5,      // max_drawdown_pct
        3         // positions_count
    )?;
    
    // 8. Update connection metrics
    println!("\n8. Updating connection metrics...");
    
    manager.update_connection_metrics(
        99.8,    // uptime_pct
        1,       // disconnection_count
        120.0,   // avg_reconnection_time_ms
        45.0,    // api_latency_ms
        15.0     // ws_latency_ms
    )?;
    
    // 9. Create and update dashboard
    println!("\n9. Creating and updating dashboard...");
    
    let dashboard_data = create_sample_dashboard_data();
    manager.update_dashboard(dashboard_data)?;
    
    // 10. Demonstrate real-time monitoring loop
    println!("\n10. Demonstrating real-time monitoring loop...");
    println!("  Starting monitoring loop for 10 seconds...");
    
    // Create a monitoring loop that runs for 10 seconds
    let start_time = std::time::Instant::now();
    let mut iteration = 0;
    
    while start_time.elapsed() < Duration::from_secs(10) {
        iteration += 1;
        println!("  Monitoring iteration {}", iteration);
        
        // Update performance metrics with slight variations
        let balance_change = (rand::random::<f64>() - 0.5) * 1000.0;
        let daily_pnl = 1500.0 + balance_change;
        let total_pnl = 5000.0 + balance_change;
        
        manager.update_performance_metrics(
            100000.0 + balance_change,
            daily_pnl,
            total_pnl,
            0.65,
            1.8,
            7.5,
            3
        )?;
        
        // Update connection metrics
        let api_latency = 45.0 + (rand::random::<f64>() - 0.5) * 10.0;
        let ws_latency = 15.0 + (rand::random::<f64>() - 0.5) * 5.0;
        
        manager.update_connection_metrics(
            99.8,
            1,
            120.0,
            api_latency,
            ws_latency
        )?;
        
        // Occasionally send alerts
        if iteration % 3 == 0 {
            let alert_message = format!("Periodic system check #{}", iteration);
            manager.send_alert(AlertLevel::Info, &alert_message, None, None)?;
        }
        
        // Update dashboard
        if manager.should_update_dashboard() {
            let mut dashboard = create_sample_dashboard_data();
            dashboard.account_summary.total_equity += balance_change;
            dashboard.account_summary.unrealized_pnl += balance_change * 0.7;
            dashboard.account_summary.realized_pnl += balance_change * 0.3;
            
            manager.update_dashboard(dashboard)?;
        }
        
        // Sleep for a second
        sleep(Duration::from_secs(1)).await;
    }
    
    // 11. Demonstrate alert escalation system
    println!("\n11. Demonstrating alert escalation system...");
    
    // Low severity alert
    manager.send_alert(AlertLevel::Info, "Minor network latency detected", None, None)?;
    
    // Medium severity alert
    manager.send_alert(AlertLevel::Warning, "Order execution delayed by 500ms", None, Some("order4"))?;
    
    // High severity alert
    manager.send_alert(AlertLevel::Error, "Failed to place order due to API error", Some("BTC"), Some("order5"))?;
    
    // Critical severity alert
    manager.send_alert(AlertLevel::Critical, "Account margin below 10% - emergency stop activated", None, None)?;
    
    // 12. Demonstrate monitoring dashboard sections
    println!("\n12. Demonstrating monitoring dashboard sections...");
    println!("  A complete monitoring dashboard includes:");
    println!("  - Account summary (balance, margin, PnL)");
    println!("  - Position summary (open positions, values, PnL)");
    println!("  - Order summary (active orders, fill rates)");
    println!("  - Risk metrics (drawdown, VaR, volatility)");
    println!("  - System status (connections, latency)");
    println!("  - Recent alerts (by severity level)");
    println!("  - Performance metrics (daily/weekly/monthly PnL)");
    
    // 13. Clean up
    println!("\n13. Cleaning up...");
    
    // Disconnect client
    client.disconnect().await?;
    println!("  Monitoring client disconnected");
    
    // Stop server
    server.stop().await?;
    println!("  Monitoring server stopped");
    
    // Print summary
    println!("\nReal-time monitoring example completed successfully!");
    println!("Total alerts sent: {}", *alert_count.lock().unwrap());
    
    Ok(())
}

fn create_sample_dashboard_data() -> MonitoringDashboardData {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Create sample risk allocation
    let mut risk_allocation = std::collections::HashMap::new();
    risk_allocation.insert("BTC".to_string(), 60.0);
    risk_allocation.insert("ETH".to_string(), 30.0);
    risk_allocation.insert("SOL".to_string(), 10.0);
    
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
            total_equity: 100000.0,
            available_balance: 70000.0,
            margin_used: 30000.0,
            margin_usage_pct: 30.0,
            current_leverage: 1.5,
            unrealized_pnl: 3000.0,
            realized_pnl: 2000.0,
            funding_pnl: 500.0,
        },
        position_summary: PositionSummary {
            open_positions: 3,
            long_positions: 2,
            short_positions: 1,
            total_position_value: 60000.0,
            largest_position: PositionSnapshot {
                symbol: "BTC".to_string(),
                size: 1.0,
                entry_price: 50000.0,
                current_price: 51000.0,
                unrealized_pnl: 1000.0,
                unrealized_pnl_pct: 2.0,
                funding_pnl: 200.0,
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
                funding_pnl: 200.0,
                liquidation_price: None,
                side: OrderSide::Buy,
                position_age_hours: 24.0,
            },
            least_profitable: PositionSnapshot {
                symbol: "SOL".to_string(),
                size: -100.0,
                entry_price: 100.0,
                current_price: 102.0,
                unrealized_pnl: -200.0,
                unrealized_pnl_pct: -2.0,
                funding_pnl: -50.0,
                liquidation_price: None,
                side: OrderSide::Sell,
                position_age_hours: 12.0,
            },
        },
        order_summary: OrderSummary {
            active_orders: 5,
            filled_today: 10,
            cancelled_today: 2,
            rejected_today: 1,
            success_rate: 0.77,
            avg_fill_time_ms: 120.0,
            volume_today: 150000.0,
            fees_today: 75.0,
        },
        risk_summary: RiskSummary {
            current_drawdown_pct: 3.0,
            max_drawdown_pct: 7.5,
            value_at_risk: 5000.0,
            daily_volatility: 2.0,
            risk_allocation,
            risk_warnings: Vec::new(),
            circuit_breaker_status: "Normal".to_string(),
        },
        system_status: SystemStatus {
            connection_status: "Connected".to_string(),
            api_latency_ms: 45.0,
            ws_latency_ms: 15.0,
            uptime_hours: 48.0,
            memory_usage_mb: 120.0,
            cpu_usage_pct: 5.0,
            last_error: None,
            last_error_time: None,
        },
        recent_alerts,
        performance: PerformanceSnapshot {
            daily_pnl: 1500.0,
            daily_pnl_pct: 1.5,
            weekly_pnl: 3500.0,
            weekly_pnl_pct: 3.5,
            monthly_pnl: 8000.0,
            monthly_pnl_pct: 8.0,
            sharpe_ratio: 1.8,
            sortino_ratio: 2.2,
            win_rate: 0.65,
            avg_win: 500.0,
            avg_loss: -300.0,
            profit_factor: 1.67,
        },
    }
}