use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use chrono::{Utc, FixedOffset};
use tokio::time::sleep;

use hyperliquid_backtest::prelude::*;
use hyperliquid_backtest::real_time_monitoring::{
    MonitoringServer, MonitoringClient, MonitoringManager,
    MonitoringMessage
};
use hyperliquid_backtest::mode_reporting::{
    MonitoringDashboardData, AlertEntry,
    AccountSummary, PositionSummary, OrderSummary, RiskSummary, SystemStatus, PerformanceSnapshot
};
use hyperliquid_backtest::live_trading::AlertLevel;
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
async fn main() -> Result<()> {
    println!("Real-Time Monitoring and Alerting Setup Example");
    println!("==============================================\n");

    // 1. Set up monitoring server
    println!("1. Setting up monitoring server...");
    let port = 8080;
    let mut server = MonitoringServer::new(port);
    server.start().await.map_err(|e| HyperliquidBacktestError::api_error(&format!("Server start error: {}", e)))?;
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
                println!("  Client received PnL update: ${:.2}", pnl.current_balance);
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
    client.connect().await.map_err(|e| HyperliquidBacktestError::api_error(&format!("Client connect error: {}", e)))?;
    println!("  Monitoring client connected");
    
    // 5. Send different types of alerts
    println!("\n5. Sending different types of alerts...");
    
    // Info alert
    manager.send_alert(AlertLevel::Info, "System started successfully", None, None)
        .map_err(|e| HyperliquidBacktestError::api_error(&format!("Send alert error: {}", e)))?;
    
    // Warning alert
    manager.send_alert(AlertLevel::Warning, "High volatility detected on BTC", Some("BTC"), None)
        .map_err(|e| HyperliquidBacktestError::api_error(&format!("Send alert error: {}", e)))?;
    
    // Error alert
    manager.send_alert(AlertLevel::Error, "API rate limit exceeded", None, None)
        .map_err(|e| HyperliquidBacktestError::api_error(&format!("Send alert error: {}", e)))?;
    
    // Critical alert
    manager.send_alert(AlertLevel::Critical, "Margin call imminent on ETH position", Some("ETH"), Some("order123"))
        .map_err(|e| HyperliquidBacktestError::api_error(&format!("Send alert error: {}", e)))?;
    
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
                time_in_force: TimeInForce::GoodTillCancel,
                stop_price: None,
                client_order_id: None,
                parameters: HashMap::new(),
            },
            OrderResult {
                order_id: "order1".to_string(),
                symbol: "BTC".to_string(),
                side: OrderSide::Buy,
                order_type: OrderType::Market,
                requested_quantity: 1.0,
                filled_quantity: 1.0,
                average_price: Some(50000.0),
                status: OrderStatus::Filled,
                timestamp: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
                fees: Some(25.0),
                error: None,
                client_order_id: None,
                metadata: HashMap::new(),
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
                time_in_force: TimeInForce::GoodTillCancel,
                stop_price: None,
                client_order_id: None,
                parameters: HashMap::new(),
            },
            OrderResult {
                order_id: "order2".to_string(),
                symbol: "ETH".to_string(),
                side: OrderSide::Buy,
                order_type: OrderType::Limit,
                requested_quantity: 10.0,
                filled_quantity: 5.0,
                average_price: Some(3000.0),
                status: OrderStatus::PartiallyFilled,
                timestamp: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
                fees: Some(7.5),
                error: None,
                client_order_id: None,
                metadata: HashMap::new(),
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
                time_in_force: TimeInForce::GoodTillCancel,
                stop_price: None,
                client_order_id: None,
                parameters: HashMap::new(),
            },
            OrderResult {
                order_id: "order3".to_string(),
                symbol: "SOL".to_string(),
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                requested_quantity: 100.0,
                filled_quantity: 0.0,
                average_price: None,
                status: OrderStatus::Rejected,
                timestamp: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
                fees: None,
                error: Some("Insufficient balance".to_string()),
                client_order_id: None,
                metadata: HashMap::new(),
            },
            50, // 50ms latency
        ),
    ];
    
    for (request, result, latency) in orders {
        manager.record_trade_execution(&request, &result, latency)
            .map_err(|e| HyperliquidBacktestError::api_error(&format!("Record trade execution error: {}", e)))?;
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
    ).map_err(|e| HyperliquidBacktestError::api_error(&format!("Update performance metrics error: {}", e)))?;
    
    // 8. Update connection metrics
    println!("\n8. Updating connection metrics...");
    
    manager.update_connection_metrics(
        99.8,    // uptime_pct
        1,       // disconnection_count
        120.0,   // avg_reconnection_time_ms
        45.0,    // api_latency_ms
        15.0     // ws_latency_ms
    ).map_err(|e| HyperliquidBacktestError::api_error(&format!("Update connection metrics error: {}", e)))?;
    
    // 9. Create and update dashboard
    println!("\n9. Creating and updating dashboard...");
    
    let dashboard_data = create_sample_dashboard_data();
    manager.update_dashboard(dashboard_data)
        .map_err(|e| HyperliquidBacktestError::api_error(&format!("Update dashboard error: {}", e)))?;
    
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
        let balance_change = (iteration as f64 * 0.1) % 1000.0 - 500.0; // Simple variation without rand
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
        ).map_err(|e| HyperliquidBacktestError::api_error(&format!("Update performance metrics error: {}", e)))?;
        
        // Update connection metrics
        let api_latency = 45.0 + (iteration as f64 * 0.5) % 10.0 - 5.0;
        let ws_latency = 15.0 + (iteration as f64 * 0.2) % 5.0 - 2.5;
        
        manager.update_connection_metrics(
            99.8,
            1,
            120.0,
            api_latency,
            ws_latency
        ).map_err(|e| HyperliquidBacktestError::api_error(&format!("Update connection metrics error: {}", e)))?;
        
        // Occasionally send alerts
        if iteration % 3 == 0 {
            let alert_message = format!("Periodic system check #{}", iteration);
            manager.send_alert(AlertLevel::Info, &alert_message, None, None)
                .map_err(|e| HyperliquidBacktestError::api_error(&format!("Send alert error: {}", e)))?;
        }
        
        // Update dashboard
        if manager.should_update_dashboard() {
            let mut dashboard = create_sample_dashboard_data();
            dashboard.account_summary.balance += balance_change;
            dashboard.account_summary.equity += balance_change;
            
            manager.update_dashboard(dashboard)
                .map_err(|e| HyperliquidBacktestError::api_error(&format!("Update dashboard error: {}", e)))?;
        }
        
        // Sleep for a second
        sleep(Duration::from_secs(1)).await;
    }
    
    // 11. Demonstrate alert escalation system
    println!("\n11. Demonstrating alert escalation system...");
    
    // Low severity alert
    manager.send_alert(AlertLevel::Info, "Minor network latency detected", None, None)
        .map_err(|e| HyperliquidBacktestError::api_error(&format!("Send alert error: {}", e)))?;
    
    // Medium severity alert
    manager.send_alert(AlertLevel::Warning, "Order execution delayed by 500ms", None, Some("order4"))
        .map_err(|e| HyperliquidBacktestError::api_error(&format!("Send alert error: {}", e)))?;
    
    // High severity alert
    manager.send_alert(AlertLevel::Error, "Failed to place order due to API error", Some("BTC"), Some("order5"))
        .map_err(|e| HyperliquidBacktestError::api_error(&format!("Send alert error: {}", e)))?;
    
    // Critical severity alert
    manager.send_alert(AlertLevel::Critical, "Account margin below 10% - emergency stop activated", None, None)
        .map_err(|e| HyperliquidBacktestError::api_error(&format!("Send alert error: {}", e)))?;
    
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
    client.disconnect().await.map_err(|e| HyperliquidBacktestError::api_error(&format!("Client disconnect error: {}", e)))?;
    println!("  Monitoring client disconnected");
    
    // Stop server
    server.stop().await.map_err(|e| HyperliquidBacktestError::api_error(&format!("Server stop error: {}", e)))?;
    println!("  Monitoring server stopped");
    
    // Print summary
    println!("\nReal-time monitoring example completed successfully!");
    println!("Total alerts sent: {}", *alert_count.lock().unwrap());
    
    Ok(())
}

fn create_sample_dashboard_data() -> MonitoringDashboardData {
    let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
    
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
            balance: 100000.0,
            equity: 103000.0,
            margin_used: 30000.0,
            margin_available: 70000.0,
        },
        position_summary: PositionSummary {
            total_positions: 3,
            total_pnl: 3000.0,
            long_positions: 2,
            short_positions: 1,
        },
        order_summary: OrderSummary {
            active_orders: 5,
            filled_orders: 10,
            cancelled_orders: 2,
            total_volume: 150000.0,
        },
        risk_summary: RiskSummary {
            risk_level: "Medium".to_string(),
            max_drawdown: 7.5,
            var_95: 5000.0,
            leverage: 1.5,
        },
        system_status: SystemStatus {
            is_connected: true,
            is_running: true,
            uptime_seconds: 172800, // 48 hours
            last_heartbeat: now,
        },
        recent_alerts,
        performance: PerformanceSnapshot {
            total_pnl: 5000.0,
            daily_pnl: 1500.0,
            win_rate: 0.65,
            sharpe_ratio: 1.8,
            max_drawdown: 7.5,
        },
    }
}