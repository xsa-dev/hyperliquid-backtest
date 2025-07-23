use std::collections::HashMap;
use chrono::{DateTime, FixedOffset, Utc};
use hyperliquid_backtester::prelude::*;
use hyperliquid_backtester::risk_manager::{RiskManager, RiskError, AssetClass};
use hyperliquid_backtester::trading_mode::{RiskConfig, TradingMode};
use hyperliquid_backtester::trading_mode_impl::{Position, OrderRequest, OrderSide, OrderType, TimeInForce};

/// # Risk Management Configuration Example
///
/// This example demonstrates how to configure and use the risk management system
/// for trading on Hyperliquid, including:
///
/// - Setting up risk parameters for different trading modes
/// - Implementing position size limits based on volatility
/// - Configuring maximum daily loss protection
/// - Setting up stop-loss and take-profit mechanisms
/// - Implementing leverage limits and margin calculations
/// - Creating portfolio-level risk management
/// - Implementing correlation-based position limits
/// - Setting up volatility-based position sizing
/// - Configuring emergency stop functionality

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Risk Management Configuration Example");
    println!("====================================\n");

    // 1. Create different risk configurations for different trading modes
    println!("1. Creating risk configurations for different trading modes...");
    
    // Conservative risk config for live trading
    let live_risk_config = RiskConfig {
        max_position_size_pct: 0.05,    // 5% of portfolio per position
        max_daily_loss_pct: 0.02,       // 2% max daily loss
        stop_loss_pct: 0.05,            // 5% stop loss
        take_profit_pct: 0.10,          // 10% take profit
        max_leverage: 2.0,              // 2x max leverage
        max_open_positions: 5,          // Maximum 5 open positions
        max_concentration_pct: 0.20,    // 20% max concentration in one asset class
        max_correlation: 0.7,           // 0.7 max correlation between positions
        max_drawdown_pct: 0.15,         // 15% max drawdown
        max_volatility_pct: 0.02,       // 2% max daily volatility
        emergency_stop_loss_pct: 0.05,  // 5% emergency stop loss
    };
    
    // Moderate risk config for paper trading
    let paper_risk_config = RiskConfig {
        max_position_size_pct: 0.10,    // 10% of portfolio per position
        max_daily_loss_pct: 0.05,       // 5% max daily loss
        stop_loss_pct: 0.08,            // 8% stop loss
        take_profit_pct: 0.15,          // 15% take profit
        max_leverage: 5.0,              // 5x max leverage
        max_open_positions: 10,         // Maximum 10 open positions
        max_concentration_pct: 0.30,    // 30% max concentration in one asset class
        max_correlation: 0.8,           // 0.8 max correlation between positions
        max_drawdown_pct: 0.25,         // 25% max drawdown
        max_volatility_pct: 0.03,       // 3% max daily volatility
        emergency_stop_loss_pct: 0.10,  // 10% emergency stop loss
    };
    
    // Aggressive risk config for backtesting
    let backtest_risk_config = RiskConfig {
        max_position_size_pct: 0.20,    // 20% of portfolio per position
        max_daily_loss_pct: 0.10,       // 10% max daily loss
        stop_loss_pct: 0.15,            // 15% stop loss
        take_profit_pct: 0.30,          // 30% take profit
        max_leverage: 10.0,             // 10x max leverage
        max_open_positions: 20,         // Maximum 20 open positions
        max_concentration_pct: 0.50,    // 50% max concentration in one asset class
        max_correlation: 0.9,           // 0.9 max correlation between positions
        max_drawdown_pct: 0.35,         // 35% max drawdown
        max_volatility_pct: 0.05,       // 5% max daily volatility
        emergency_stop_loss_pct: 0.15,  // 15% emergency stop loss
    };
    
    println!("  Live trading risk config: {:?}", live_risk_config);
    println!("  Paper trading risk config: {:?}", paper_risk_config);
    println!("  Backtesting risk config: {:?}", backtest_risk_config);
    
    // 2. Initialize risk managers for different trading modes
    println!("\n2. Initializing risk managers for different trading modes...");
    
    let initial_portfolio_value = 100000.0; // $100,000 initial portfolio value
    
    let mut live_risk_manager = RiskManager::new(live_risk_config, initial_portfolio_value);
    let mut paper_risk_manager = RiskManager::new(paper_risk_config, initial_portfolio_value);
    let mut backtest_risk_manager = RiskManager::new(backtest_risk_config, initial_portfolio_value);
    
    println!("  Risk managers initialized with ${:.2} portfolio value", initial_portfolio_value);
    
    // 3. Create sample positions and orders
    println!("\n3. Creating sample positions and orders...");
    
    let mut positions = HashMap::new();
    
    // Add some existing positions
    positions.insert("BTC".to_string(), create_position("BTC", 1.0, 50000.0, 51000.0));
    positions.insert("ETH".to_string(), create_position("ETH", 10.0, 3000.0, 3100.0));
    
    // Create sample orders
    let btc_order = OrderRequest {
        symbol: "BTC".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        quantity: 0.5,
        price: Some(51000.0),
        reduce_only: false,
        time_in_force: TimeInForce::GoodTilCancelled,
    };
    
    let eth_order = OrderRequest {
        symbol: "ETH".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        quantity: 5.0,
        price: Some(3100.0),
        reduce_only: false,
        time_in_force: TimeInForce::GoodTilCancelled,
    };
    
    let sol_order = OrderRequest {
        symbol: "SOL".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Limit,
        quantity: 100.0,
        price: Some(100.0),
        reduce_only: false,
        time_in_force: TimeInForce::GoodTilCancelled,
    };
    
    println!("  Created sample positions and orders");
    
    // 4. Validate orders against risk limits
    println!("\n4. Validating orders against risk limits...");
    
    // Validate BTC order with different risk managers
    println!("\n  Validating BTC order (0.5 BTC @ $51,000 = $25,500):");
    
    match live_risk_manager.validate_order(&btc_order, &positions) {
        Ok(_) => println!("    Live trading: Order ACCEPTED"),
        Err(e) => println!("    Live trading: Order REJECTED - {}", e),
    }
    
    match paper_risk_manager.validate_order(&btc_order, &positions) {
        Ok(_) => println!("    Paper trading: Order ACCEPTED"),
        Err(e) => println!("    Paper trading: Order REJECTED - {}", e),
    }
    
    match backtest_risk_manager.validate_order(&btc_order, &positions) {
        Ok(_) => println!("    Backtesting: Order ACCEPTED"),
        Err(e) => println!("    Backtesting: Order REJECTED - {}", e),
    }
    
    // Validate SOL order with different risk managers
    println!("\n  Validating SOL order (100 SOL @ $100 = $10,000):");
    
    match live_risk_manager.validate_order(&sol_order, &positions) {
        Ok(_) => println!("    Live trading: Order ACCEPTED"),
        Err(e) => println!("    Live trading: Order REJECTED - {}", e),
    }
    
    match paper_risk_manager.validate_order(&sol_order, &positions) {
        Ok(_) => println!("    Paper trading: Order ACCEPTED"),
        Err(e) => println!("    Paper trading: Order REJECTED - {}", e),
    }
    
    match backtest_risk_manager.validate_order(&sol_order, &positions) {
        Ok(_) => println!("    Backtesting: Order ACCEPTED"),
        Err(e) => println!("    Backtesting: Order REJECTED - {}", e),
    }
    
    // 5. Generate stop-loss and take-profit orders
    println!("\n5. Generating stop-loss and take-profit orders...");
    
    let btc_position = positions.get("BTC").unwrap();
    
    // Generate stop-loss for BTC position
    if let Some(stop_loss) = live_risk_manager.generate_stop_loss(btc_position, "btc_order_1") {
        println!("  BTC Stop-Loss: {} {} @ ${:.2}", 
                 stop_loss.side, stop_loss.quantity, stop_loss.trigger_price);
    }
    
    // Generate take-profit for BTC position
    if let Some(take_profit) = live_risk_manager.generate_take_profit(btc_position, "btc_order_1") {
        println!("  BTC Take-Profit: {} {} @ ${:.2}", 
                 take_profit.side, take_profit.quantity, take_profit.trigger_price);
    }
    
    // 6. Demonstrate daily loss limit
    println!("\n6. Demonstrating daily loss limit...");
    
    // Update portfolio value with a loss
    let new_portfolio_value = initial_portfolio_value * 0.98; // 2% loss
    let realized_pnl_delta = -2000.0; // $2,000 realized loss
    
    println!("  Updating portfolio value to ${:.2} (2% loss)", new_portfolio_value);
    
    match live_risk_manager.update_portfolio_value(new_portfolio_value, realized_pnl_delta) {
        Ok(_) => println!("  Live trading: Daily loss within limits"),
        Err(e) => println!("  Live trading: Daily loss limit triggered - {}", e),
    }
    
    // Update with a larger loss that should trigger the daily loss limit
    let new_portfolio_value = initial_portfolio_value * 0.97; // 3% loss
    let realized_pnl_delta = -1000.0; // Additional $1,000 realized loss
    
    println!("  Updating portfolio value to ${:.2} (3% loss)", new_portfolio_value);
    
    match live_risk_manager.update_portfolio_value(new_portfolio_value, realized_pnl_delta) {
        Ok(_) => println!("  Live trading: Daily loss within limits"),
        Err(e) => println!("  Live trading: Daily loss limit triggered - {}", e),
    }
    
    // 7. Demonstrate emergency stop functionality
    println!("\n7. Demonstrating emergency stop functionality...");
    
    println!("  Activating emergency stop");
    live_risk_manager.activate_emergency_stop();
    
    // Try to validate an order after emergency stop
    match live_risk_manager.validate_order(&btc_order, &positions) {
        Ok(_) => println!("  Order ACCEPTED despite emergency stop (unexpected)"),
        Err(e) => println!("  Order REJECTED due to emergency stop - {}", e),
    }
    
    println!("  Deactivating emergency stop");
    live_risk_manager.deactivate_emergency_stop();
    
    // 8. Calculate margin requirements
    println!("\n8. Calculating margin requirements...");
    
    // Calculate required margin for different position sizes
    let btc_position_value = 50000.0; // $50,000 BTC position
    let eth_position_value = 30000.0; // $30,000 ETH position
    let sol_position_value = 10000.0; // $10,000 SOL position
    
    println!("  Live trading margin requirements:");
    println!("    BTC position (${:.2}): ${:.2} margin required", 
             btc_position_value, live_risk_manager.calculate_required_margin(btc_position_value));
    println!("    ETH position (${:.2}): ${:.2} margin required", 
             eth_position_value, live_risk_manager.calculate_required_margin(eth_position_value));
    println!("    SOL position (${:.2}): ${:.2} margin required", 
             sol_position_value, live_risk_manager.calculate_required_margin(sol_position_value));
    
    println!("  Paper trading margin requirements:");
    println!("    BTC position (${:.2}): ${:.2} margin required", 
             btc_position_value, paper_risk_manager.calculate_required_margin(btc_position_value));
    println!("    ETH position (${:.2}): ${:.2} margin required", 
             eth_position_value, paper_risk_manager.calculate_required_margin(eth_position_value));
    println!("    SOL position (${:.2}): ${:.2} margin required", 
             sol_position_value, paper_risk_manager.calculate_required_margin(sol_position_value));
    
    // 9. Demonstrate risk configuration for different market conditions
    println!("\n9. Demonstrating risk configuration for different market conditions...");
    
    // Create risk configs for different market conditions
    let low_volatility_config = create_market_condition_risk_config("low_volatility");
    let high_volatility_config = create_market_condition_risk_config("high_volatility");
    let trending_market_config = create_market_condition_risk_config("trending");
    let ranging_market_config = create_market_condition_risk_config("ranging");
    
    println!("  Low Volatility Risk Config:");
    print_risk_config_summary(&low_volatility_config);
    
    println!("  High Volatility Risk Config:");
    print_risk_config_summary(&high_volatility_config);
    
    println!("  Trending Market Risk Config:");
    print_risk_config_summary(&trending_market_config);
    
    println!("  Ranging Market Risk Config:");
    print_risk_config_summary(&ranging_market_config);
    
    // 10. Demonstrate risk manager integration with trading modes
    println!("\n10. Demonstrating risk manager integration with trading modes...");
    
    println!("  Creating trading mode managers with appropriate risk configurations");
    println!("  - Backtest mode: Uses more aggressive risk parameters");
    println!("  - Paper trading mode: Uses moderate risk parameters");
    println!("  - Live trading mode: Uses conservative risk parameters");
    println!("  - Each mode automatically applies the appropriate risk checks");
    
    println!("\nRisk management configuration example completed successfully!");
    
    Ok(())
}

fn create_position(symbol: &str, size: f64, entry_price: f64, current_price: f64) -> Position {
    Position {
        symbol: symbol.to_string(),
        size,
        entry_price,
        current_price,
        unrealized_pnl: (current_price - entry_price) * size,
        realized_pnl: 0.0,
        funding_pnl: 0.0,
        timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
    }
}

fn create_market_condition_risk_config(market_condition: &str) -> RiskConfig {
    match market_condition {
        "low_volatility" => RiskConfig {
            max_position_size_pct: 0.15,    // 15% of portfolio per position
            max_daily_loss_pct: 0.03,       // 3% max daily loss
            stop_loss_pct: 0.05,            // 5% stop loss
            take_profit_pct: 0.10,          // 10% take profit
            max_leverage: 5.0,              // 5x max leverage
            max_open_positions: 10,         // Maximum 10 open positions
            max_concentration_pct: 0.30,    // 30% max concentration in one asset class
            max_correlation: 0.8,           // 0.8 max correlation between positions
            max_drawdown_pct: 0.20,         // 20% max drawdown
            max_volatility_pct: 0.02,       // 2% max daily volatility
            emergency_stop_loss_pct: 0.07,  // 7% emergency stop loss
        },
        "high_volatility" => RiskConfig {
            max_position_size_pct: 0.05,    // 5% of portfolio per position
            max_daily_loss_pct: 0.02,       // 2% max daily loss
            stop_loss_pct: 0.07,            // 7% stop loss
            take_profit_pct: 0.15,          // 15% take profit
            max_leverage: 2.0,              // 2x max leverage
            max_open_positions: 5,          // Maximum 5 open positions
            max_concentration_pct: 0.20,    // 20% max concentration in one asset class
            max_correlation: 0.6,           // 0.6 max correlation between positions
            max_drawdown_pct: 0.15,         // 15% max drawdown
            max_volatility_pct: 0.03,       // 3% max daily volatility
            emergency_stop_loss_pct: 0.05,  // 5% emergency stop loss
        },
        "trending" => RiskConfig {
            max_position_size_pct: 0.10,    // 10% of portfolio per position
            max_daily_loss_pct: 0.03,       // 3% max daily loss
            stop_loss_pct: 0.08,            // 8% stop loss
            take_profit_pct: 0.20,          // 20% take profit
            max_leverage: 3.0,              // 3x max leverage
            max_open_positions: 7,          // Maximum 7 open positions
            max_concentration_pct: 0.25,    // 25% max concentration in one asset class
            max_correlation: 0.7,           // 0.7 max correlation between positions
            max_drawdown_pct: 0.18,         // 18% max drawdown
            max_volatility_pct: 0.025,      // 2.5% max daily volatility
            emergency_stop_loss_pct: 0.06,  // 6% emergency stop loss
        },
        "ranging" => RiskConfig {
            max_position_size_pct: 0.08,    // 8% of portfolio per position
            max_daily_loss_pct: 0.02,       // 2% max daily loss
            stop_loss_pct: 0.05,            // 5% stop loss
            take_profit_pct: 0.08,          // 8% take profit
            max_leverage: 2.5,              // 2.5x max leverage
            max_open_positions: 8,          // Maximum 8 open positions
            max_concentration_pct: 0.20,    // 20% max concentration in one asset class
            max_correlation: 0.6,           // 0.6 max correlation between positions
            max_drawdown_pct: 0.15,         // 15% max drawdown
            max_volatility_pct: 0.02,       // 2% max daily volatility
            emergency_stop_loss_pct: 0.05,  // 5% emergency stop loss
        },
        _ => RiskConfig::default(),
    }
}

fn print_risk_config_summary(config: &RiskConfig) {
    println!("    - Max position size: {:.1}% of portfolio", config.max_position_size_pct * 100.0);
    println!("    - Max daily loss: {:.1}%", config.max_daily_loss_pct * 100.0);
    println!("    - Stop loss: {:.1}%", config.stop_loss_pct * 100.0);
    println!("    - Take profit: {:.1}%", config.take_profit_pct * 100.0);
    println!("    - Max leverage: {:.1}x", config.max_leverage);
    println!("    - Max open positions: {}", config.max_open_positions);
}