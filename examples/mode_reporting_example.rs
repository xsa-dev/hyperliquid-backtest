use std::collections::HashMap;
use chrono::{DateTime, FixedOffset, Utc};
use hyperliquid_backtest::{
    mode_reporting::{
        ModeReportingManager, CommonPerformanceMetrics, FundingImpactAnalysis,
        RiskMetrics, ConnectionMetrics, AlertEntry, OrderSummary
    },
    trading_mode::TradingMode,
    unified_data::{Position, OrderSide, OrderType},
    paper_trading::TradeLogEntry,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Mode-specific Reporting Example");
    println!("===============================\n");
    
    // Create a reporting manager for paper trading
    let mut paper_manager = ModeReportingManager::new(TradingMode::PaperTrade, 10000.0);
    
    // Create a reporting manager for live trading
    let mut live_manager = ModeReportingManager::new(TradingMode::LiveTrade, 10000.0);
    
    // Create test positions
    let mut positions = HashMap::new();
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // BTC long position
    let mut btc_position = Position::new("BTC", 1.0, 50000.0, 52000.0, now);
    btc_position.unrealized_pnl = 2000.0;  // 1.0 * (52000 - 50000)
    btc_position.realized_pnl = 500.0;
    btc_position.funding_pnl = 50.0;
    positions.insert("BTC".to_string(), btc_position);
    
    // ETH short position
    let mut eth_position = Position::new("ETH", -10.0, 3000.0, 2900.0, now);
    eth_position.unrealized_pnl = 1000.0;  // -10.0 * (2900 - 3000)
    eth_position.realized_pnl = 300.0;
    eth_position.funding_pnl = -20.0;
    positions.insert("ETH".to_string(), eth_position);
    
    // Create trade log
    let mut trade_log = Vec::new();
    trade_log.push(TradeLogEntry {
        id: "1".to_string(),
        symbol: "BTC".to_string(),
        side: OrderSide::Buy,
        quantity: 1.0,
        price: 50000.0,
        timestamp: now - chrono::Duration::days(5),
        fees: 25.0,
        order_type: OrderType::Market,
        order_id: "order1".to_string(),
        pnl: None,
        metadata: HashMap::new(),
    });
    
    trade_log.push(TradeLogEntry {
        id: "2".to_string(),
        symbol: "ETH".to_string(),
        side: OrderSide::Sell,
        quantity: 10.0,
        price: 3000.0,
        timestamp: now - chrono::Duration::days(3),
        fees: 15.0,
        order_type: OrderType::Limit,
        order_id: "order2".to_string(),
        pnl: Some(300.0),
        metadata: HashMap::new(),
    });
    
    // Create performance metrics
    let paper_metrics = CommonPerformanceMetrics {
        mode: TradingMode::PaperTrade,
        initial_balance: 10000.0,
        current_balance: 10780.0,
        realized_pnl: 800.0,
        unrealized_pnl: 3000.0,
        funding_pnl: 30.0,
        total_pnl: 3830.0,
        total_fees: 40.0,
        total_return_pct: 38.3,
        trade_count: 2,
        win_rate: 100.0,
        max_drawdown: 200.0,
        max_drawdown_pct: 2.0,
        start_time: now - chrono::Duration::days(10),
        end_time: now,
        duration_days: 10.0,
    };
    
    let live_metrics = CommonPerformanceMetrics {
        mode: TradingMode::LiveTrade,
        initial_balance: 10000.0,
        current_balance: 10700.0,
        realized_pnl: 750.0,
        unrealized_pnl: 2900.0,
        funding_pnl: 25.0,
        total_pnl: 3675.0,
        total_fees: 50.0,
        total_return_pct: 36.75,
        trade_count: 2,
        win_rate: 100.0,
        max_drawdown: 250.0,
        max_drawdown_pct: 2.5,
        start_time: now - chrono::Duration::days(10),
        end_time: now,
        duration_days: 10.0,
    };
    
    // Create funding impact analysis
    let mut funding_by_symbol = HashMap::new();
    funding_by_symbol.insert(
        "BTC".to_string(),
        hyperliquid_backtest::mode_reporting::SymbolFundingMetrics {
            symbol: "BTC".to_string(),
            funding_pnl: 50.0,
            avg_funding_rate: 0.0001,
            funding_volatility: 0.00005,
            funding_received: 60.0,
            funding_paid: 10.0,
            payment_count: 30,
        }
    );
    
    funding_by_symbol.insert(
        "ETH".to_string(),
        hyperliquid_backtest::mode_reporting::SymbolFundingMetrics {
            symbol: "ETH".to_string(),
            funding_pnl: -20.0,
            avg_funding_rate: -0.0002,
            funding_volatility: 0.0001,
            funding_received: 5.0,
            funding_paid: 25.0,
            payment_count: 30,
        }
    );
    
    let funding_impact = FundingImpactAnalysis {
        total_funding_pnl: 30.0,
        funding_pnl_percentage: 0.78,  // 30.0 / 3830.0 * 100
        avg_funding_rate: -0.00005,
        funding_rate_volatility: 0.00015,
        funding_received: 65.0,
        funding_paid: 35.0,
        payment_count: 60,
        funding_price_correlation: 0.2,
        funding_by_symbol,
    };
    
    // Create risk metrics for live trading
    let risk_metrics = RiskMetrics {
        current_leverage: 2.0,
        max_leverage: 2.5,
        value_at_risk_95: 400.0,
        value_at_risk_99: 700.0,
        expected_shortfall_95: 500.0,
        beta: 1.1,
        correlation: 0.7,
        position_concentration: 0.65,
        largest_position: 52000.0,
        largest_position_symbol: "BTC".to_string(),
    };
    
    // Create connection metrics for live trading
    let connection_metrics = ConnectionMetrics {
        uptime_pct: 99.9,
        disconnection_count: 1,
        avg_reconnection_time_ms: 120.0,
        api_latency_ms: 45.0,
        ws_latency_ms: 18.0,
        order_latency_ms: 75.0,
    };
    
    // Create alerts for live trading
    let mut alerts = Vec::new();
    alerts.push(AlertEntry {
        level: "INFO".to_string(),
        message: "Strategy started".to_string(),
        timestamp: now - chrono::Duration::hours(10),
        symbol: None,
        order_id: None,
    });
    
    alerts.push(AlertEntry {
        level: "WARNING".to_string(),
        message: "High volatility detected".to_string(),
        timestamp: now - chrono::Duration::hours(5),
        symbol: Some("BTC".to_string()),
        order_id: None,
    });
    
    // Update paper trading manager
    paper_manager.update_performance(paper_metrics);
    paper_manager.update_funding_impact(funding_impact.clone());
    
    // Update live trading manager
    live_manager.update_performance(live_metrics);
    live_manager.update_funding_impact(funding_impact);
    live_manager.update_risk_metrics(risk_metrics);
    live_manager.update_connection_metrics(connection_metrics);
    
    for alert in alerts {
        live_manager.add_alert(alert);
    }
    
    // Generate paper trading report
    let paper_report = paper_manager.generate_paper_trading_report(trade_log.clone(), positions.clone())?;
    
    // Generate live trading report
    let live_report = live_manager.generate_live_trading_report(trade_log, positions.clone())?;
    
    // Generate real-time PnL report
    let pnl_report = paper_manager.generate_real_time_pnl_report(10780.0, positions.clone())?;
    
    // Generate monitoring dashboard
    let order_summary = OrderSummary {
        active_orders: 3,
        filled_today: 8,
        cancelled_today: 1,
        rejected_today: 0,
        success_rate: 88.9,
        avg_fill_time_ms: 110.0,
        volume_today: 120000.0,
        fees_today: 60.0,
    };
    
    let dashboard = live_manager.generate_monitoring_dashboard(
        10700.0,
        8000.0,
        positions,
        3,
        order_summary
    )?;
    
    // Print paper trading report summary
    println!("Paper Trading Report");
    println!("-------------------");
    println!("Initial Balance: ${:.2}", paper_report.common.initial_balance);
    println!("Current Balance: ${:.2}", paper_report.common.current_balance);
    println!("Unrealized PnL: ${:.2}", paper_report.common.unrealized_pnl);
    println!("Realized PnL: ${:.2}", paper_report.common.realized_pnl);
    println!("Funding PnL: ${:.2}", paper_report.common.funding_pnl);
    println!("Total PnL: ${:.2}", paper_report.common.total_pnl);
    println!("Return: {:.2}%", paper_report.common.total_return_pct);
    println!("Annualized Return: {:.2}%", paper_report.annualized_return);
    println!("Sharpe Ratio: {:.2}", paper_report.sharpe_ratio);
    println!("Sortino Ratio: {:.2}", paper_report.sortino_ratio);
    println!("Max Drawdown: {:.2}%", paper_report.common.max_drawdown_pct);
    println!();
    
    // Print live trading report summary
    println!("Live Trading Report");
    println!("------------------");
    println!("Initial Balance: ${:.2}", live_report.common.initial_balance);
    println!("Current Balance: ${:.2}", live_report.common.current_balance);
    println!("Unrealized PnL: ${:.2}", live_report.common.unrealized_pnl);
    println!("Realized PnL: ${:.2}", live_report.common.realized_pnl);
    println!("Funding PnL: ${:.2}", live_report.common.funding_pnl);
    println!("Total PnL: ${:.2}", live_report.common.total_pnl);
    println!("Return: {:.2}%", live_report.common.total_return_pct);
    println!("Current Leverage: {:.2}x", live_report.risk_metrics.current_leverage);
    println!("Value at Risk (95%): ${:.2}", live_report.risk_metrics.value_at_risk_95);
    println!("Connection Uptime: {:.2}%", live_report.connection_metrics.uptime_pct);
    println!("API Latency: {:.2}ms", live_report.connection_metrics.api_latency_ms);
    println!();
    
    // Print real-time PnL report
    println!("Real-time PnL Report");
    println!("-------------------");
    println!("Current Balance: ${:.2}", pnl_report.current_balance);
    println!("Realized PnL: ${:.2}", pnl_report.realized_pnl);
    println!("Unrealized PnL: ${:.2}", pnl_report.unrealized_pnl);
    println!("Funding PnL: ${:.2}", pnl_report.funding_pnl);
    println!("Total PnL: ${:.2}", pnl_report.total_pnl);
    println!("Daily PnL: ${:.2}", pnl_report.daily_pnl);
    println!("Hourly PnL: ${:.2}", pnl_report.hourly_pnl);
    println!();
    
    // Print monitoring dashboard summary
    println!("Live Trading Dashboard");
    println!("---------------------");
    println!("Total Equity: ${:.2}", dashboard.account_summary.total_equity);
    println!("Available Balance: ${:.2}", dashboard.account_summary.available_balance);
    println!("Margin Usage: {:.2}%", dashboard.account_summary.margin_usage_pct);
    println!("Open Positions: {}", dashboard.position_summary.open_positions);
    println!("Long Positions: {}", dashboard.position_summary.long_positions);
    println!("Short Positions: {}", dashboard.position_summary.short_positions);
    println!("Active Orders: {}", dashboard.order_summary.active_orders);
    println!("Filled Today: {}", dashboard.order_summary.filled_today);
    println!("Success Rate: {:.2}%", dashboard.order_summary.success_rate);
    println!("Current Drawdown: {:.2}%", dashboard.risk_summary.current_drawdown_pct);
    println!("System Status: {}", dashboard.system_status.connection_status);
    println!();
    
    // Print funding impact analysis
    println!("Funding Impact Analysis");
    println!("----------------------");
    println!("Total Funding PnL: ${:.2}", funding_impact.total_funding_pnl);
    println!("Funding PnL % of Total: {:.2}%", funding_impact.funding_pnl_percentage);
    println!("Average Funding Rate: {:.6}%", funding_impact.avg_funding_rate * 100.0);
    println!("Funding Rate Volatility: {:.6}%", funding_impact.funding_rate_volatility * 100.0);
    println!("Funding Received: ${:.2}", funding_impact.funding_received);
    println!("Funding Paid: ${:.2}", funding_impact.funding_paid);
    println!("Funding Payments: {}", funding_impact.payment_count);
    println!();
    
    println!("BTC Funding Metrics:");
    let btc_metrics = &funding_impact.funding_by_symbol["BTC"];
    println!("  Funding PnL: ${:.2}", btc_metrics.funding_pnl);
    println!("  Avg Rate: {:.6}%", btc_metrics.avg_funding_rate * 100.0);
    println!();
    
    println!("ETH Funding Metrics:");
    let eth_metrics = &funding_impact.funding_by_symbol["ETH"];
    println!("  Funding PnL: ${:.2}", eth_metrics.funding_pnl);
    println!("  Avg Rate: {:.6}%", eth_metrics.avg_funding_rate * 100.0);
    
    Ok(())
}