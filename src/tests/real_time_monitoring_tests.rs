#[cfg(test)]
mod tests {
    use std::time::Duration;
    use chrono::{Utc, FixedOffset};
    use tokio::time::sleep;
    
    use crate::real_time_monitoring::{
        MonitoringServer, MonitoringClient, MonitoringManager,
        MonitoringMessage, TradeExecutionUpdate, ConnectionStatus
    };
    use crate::trading_mode::TradingMode;
    use crate::unified_data::{OrderRequest, OrderResult, OrderSide, OrderType, OrderStatus, TimeInForce};
    use crate::live_trading::AlertLevel;
    
    #[tokio::test]
    async fn test_monitoring_server_creation() {
        let mut server = MonitoringServer::new(8080);
        assert_eq!(server.port, 8080);
        assert_eq!(server.client_count(), 0);
        
        // Start server
        let result = server.start().await;
        assert!(result.is_ok());
        
        // Stop server
        let result = server.stop().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_monitoring_client_creation() {
        let client = MonitoringClient::new("ws://localhost:8080");
        assert_eq!(client.server_address, "ws://localhost:8080");
        assert_eq!(client.is_connected(), false);
        assert_eq!(client.connection_status(), ConnectionStatus::Disconnected);
    }
    
    #[tokio::test]
    async fn test_monitoring_manager_creation() {
        let manager = MonitoringManager::new(TradingMode::LiveTrade);
        assert_eq!(manager.mode, TradingMode::LiveTrade);
        assert_eq!(manager.get_alert_history().len(), 0);
        assert_eq!(manager.get_trade_execution_history().len(), 0);
    }
    
    #[tokio::test]
    async fn test_send_alert() {
        let mut manager = MonitoringManager::new(TradingMode::LiveTrade);
        
        // Send an alert
        let result = manager.send_alert(
            AlertLevel::Warning,
            "Test alert",
            Some("BTC"),
            None
        );
        
        assert!(result.is_ok());
        assert_eq!(manager.get_alert_history().len(), 1);
        
        let alert = &manager.get_alert_history()[0];
        assert_eq!(alert.level, "Warning");
        assert_eq!(alert.message, "Test alert");
        assert_eq!(alert.symbol, Some("BTC".to_string()));
        assert_eq!(alert.order_id, None);
    }
    
    #[tokio::test]
    async fn test_record_trade_execution() {
        let mut manager = MonitoringManager::new(TradingMode::LiveTrade);
        
        // Create order request and result
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
            order_id: "test_order".to_string(),
            status: OrderStatus::Filled,
            filled_quantity: 1.0,
            average_price: Some(50000.0),
            fees: Some(25.0),
            timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
            error: None,
        };
        
        // Record trade execution
        let result = manager.record_trade_execution(&order_request, &order_result, 100);
        
        assert!(result.is_ok());
        assert_eq!(manager.get_trade_execution_history().len(), 1);
        
        let execution = &manager.get_trade_execution_history()[0];
        assert_eq!(execution.order_id, "test_order");
        assert_eq!(execution.symbol, "BTC");
        assert_eq!(execution.status, OrderStatus::Filled);
        assert_eq!(execution.filled_quantity, 1.0);
        assert_eq!(execution.average_price, Some(50000.0));
        assert_eq!(execution.execution_latency_ms, 100);
        assert_eq!(execution.error, None);
    }
    
    #[tokio::test]
    async fn test_update_performance_metrics() {
        let mut manager = MonitoringManager::new(TradingMode::LiveTrade);
        
        // Update performance metrics
        let result = manager.update_performance_metrics(
            10000.0, // current_balance
            100.0,   // daily_pnl
            500.0,   // total_pnl
            0.6,     // win_rate
            1.5,     // sharpe_ratio
            5.0,     // max_drawdown_pct
            2        // positions_count
        );
        
        assert!(result.is_ok());
        assert_eq!(manager.get_performance_metrics_history().len(), 1);
        
        let metrics = &manager.get_performance_metrics_history()[0];
        assert_eq!(metrics.current_balance, 10000.0);
        assert_eq!(metrics.daily_pnl, 100.0);
        assert_eq!(metrics.daily_pnl_pct, 1.0); // 100 / 10000 * 100
        assert_eq!(metrics.total_pnl, 500.0);
        assert_eq!(metrics.total_return_pct, 5.0); // 500 / 10000 * 100
        assert_eq!(metrics.win_rate, 0.6);
        assert_eq!(metrics.sharpe_ratio, 1.5);
        assert_eq!(metrics.max_drawdown_pct, 5.0);
        assert_eq!(metrics.positions_count, 2);
    }
    
    #[tokio::test]
    async fn test_update_connection_metrics() {
        let mut manager = MonitoringManager::new(TradingMode::LiveTrade);
        
        // Update connection metrics
        let result = manager.update_connection_metrics(
            99.5,    // uptime_pct
            2,       // disconnection_count
            150.0,   // avg_reconnection_time_ms
            50.0,    // api_latency_ms
            25.0     // ws_latency_ms
        );
        
        assert!(result.is_ok());
        
        let metrics = manager.get_connection_metrics();
        assert_eq!(metrics.uptime_pct, 99.5);
        assert_eq!(metrics.disconnection_count, 2);
        assert_eq!(metrics.avg_reconnection_time_ms, 150.0);
        assert_eq!(metrics.api_latency_ms, 50.0);
        assert_eq!(metrics.ws_latency_ms, 25.0);
    }
    
    #[tokio::test]
    async fn test_message_handlers() {
        let mut manager = MonitoringManager::new(TradingMode::LiveTrade);
        
        // Add alert handler
        let alert_received = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let alert_received_clone = alert_received.clone();
        
        manager.add_alert_handler(move |alert| {
            if alert.message == "Test alert" {
                alert_received_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });
        
        // Send alert
        manager.send_alert(AlertLevel::Info, "Test alert", None, None).unwrap();
        
        // Check that handler was called
        assert!(alert_received.load(std::sync::atomic::Ordering::SeqCst));
        
        // Add trade execution handler
        let execution_received = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let execution_received_clone = execution_received.clone();
        
        manager.add_trade_execution_handler(move |execution| {
            if execution.order_id == "test_order" {
                execution_received_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });
        
        // Create order request and result
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
            order_id: "test_order".to_string(),
            status: OrderStatus::Filled,
            filled_quantity: 1.0,
            average_price: Some(50000.0),
            fees: Some(25.0),
            timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
            error: None,
        };
        
        // Record trade execution
        manager.record_trade_execution(&order_request, &order_result, 100).unwrap();
        
        // Check that handler was called
        assert!(execution_received.load(std::sync::atomic::Ordering::SeqCst));
    }
    
    #[tokio::test]
    async fn test_dashboard_update_interval() {
        let mut manager = MonitoringManager::new(TradingMode::LiveTrade);
        
        // Set dashboard update interval to 1 second
        manager.set_dashboard_update_interval(1);
        
        // Should update dashboard initially
        assert!(manager.should_update_dashboard());
        
        // Simulate dashboard update
        let dashboard_data = crate::mode_reporting::MonitoringDashboardData {
            timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
            account_summary: crate::mode_reporting::AccountSummary {
                total_equity: 10000.0,
                available_balance: 9000.0,
                margin_used: 1000.0,
                margin_usage_pct: 10.0,
                current_leverage: 1.0,
                unrealized_pnl: 100.0,
                realized_pnl: 200.0,
                funding_pnl: 50.0,
            },
            position_summary: crate::mode_reporting::PositionSummary {
                open_positions: 2,
                long_positions: 1,
                short_positions: 1,
                total_position_value: 10000.0,
                largest_position: crate::mode_reporting::PositionSnapshot {
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
                most_profitable: crate::mode_reporting::PositionSnapshot {
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
                least_profitable: crate::mode_reporting::PositionSnapshot {
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
            order_summary: crate::mode_reporting::OrderSummary {
                active_orders: 1,
                filled_today: 5,
                cancelled_today: 2,
                rejected_today: 0,
                success_rate: 0.8,
                avg_fill_time_ms: 120.0,
                volume_today: 50000.0,
                fees_today: 25.0,
            },
            risk_summary: crate::mode_reporting::RiskSummary {
                current_drawdown_pct: 2.0,
                max_drawdown_pct: 5.0,
                value_at_risk: 500.0,
                daily_volatility: 1.5,
                risk_allocation: std::collections::HashMap::new(),
                risk_warnings: Vec::new(),
                circuit_breaker_status: "Normal".to_string(),
            },
            system_status: crate::mode_reporting::SystemStatus {
                connection_status: "Connected".to_string(),
                api_latency_ms: 50.0,
                ws_latency_ms: 25.0,
                uptime_hours: 24.0,
                memory_usage_mb: 100.0,
                cpu_usage_pct: 5.0,
                last_error: None,
                last_error_time: None,
            },
            recent_alerts: Vec::new(),
            performance: crate::mode_reporting::PerformanceSnapshot {
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
        };
        
        manager.update_dashboard(dashboard_data).unwrap();
        
        // Should not update dashboard immediately after update
        assert!(!manager.should_update_dashboard());
        
        // Wait for interval to pass
        sleep(Duration::from_secs(2)).await;
        
        // Should update dashboard after interval
        assert!(manager.should_update_dashboard());
    }
}