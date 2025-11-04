use hyperliquid_backtest::prelude::*;
use chrono::{Utc, FixedOffset};
use std::collections::HashMap;

/// # Simple Working Example
///
/// This example demonstrates the basic functionality available in the current library.
/// It shows how to create positions, orders, and use the risk manager.

fn main() -> Result<()> {
    println!("🚀 Simple Hyperliquid Backtester Example");
    println!("========================================\n");

    // Create a position
    println!("📊 Creating a position...");
    let mut position = Position::new(
        "BTC",
        1.0,
        50000.0,
        51000.0, // current price
        Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
    );
    
    println!("   Position: {} {} at ${:.2} (current: ${:.2})", 
        position.size, position.symbol, position.entry_price, position.current_price);
    println!("   Unrealized PnL: ${:.2}", position.unrealized_pnl());

    // Create an order request
    println!("\n📝 Creating an order request...");
    let order = OrderRequest::market("BTC", OrderSide::Buy, 0.5);
    
    println!("   Order: {:?} {} {} at market price", 
        order.side, order.quantity, order.symbol);

    // Create a risk manager
    println!("\n🛡️ Setting up risk management...");
    let risk_config = RiskConfig {
        max_position_size_pct: 0.1, // 10% of portfolio
        stop_loss_pct: 0.05,        // 5% stop loss
        take_profit_pct: 0.1,       // 10% take profit
    };
    
    let risk_manager = RiskManager::new(risk_config, 10000.0); // $10,000 portfolio
    
    println!("   Max position size: {:.1}% of portfolio", risk_manager.config().max_position_size_pct * 100.0);
    println!("   Stop loss: {:.1}%", risk_manager.config().stop_loss_pct * 100.0);
    println!("   Take profit: {:.1}%", risk_manager.config().take_profit_pct * 100.0);

    // Check if the order is allowed
    println!("\n🔍 Checking if order is allowed...");
    let mut positions = HashMap::new();
    positions.insert("BTC".to_string(), position.clone());
    
    match risk_manager.validate_order(&order, &positions) {
        Ok(_) => println!("   ✅ Order is allowed by risk manager"),
        Err(e) => println!("   ❌ Order rejected: {}", e),
    }

    // Simulate order execution
    println!("\n⚡ Simulating order execution...");
    let order_result = OrderResult::new(
        "12345",
        "BTC",
        order.side,
        order.quantity,
        51000.0, // execution price
    );
    
    println!("   Order executed: {:?} {} {} at ${:.2}", 
        order_result.side, order_result.quantity, order_result.symbol, order_result.price);

    // Update position
    println!("\n📈 Updating position...");
    position.size += order_result.quantity;
    position.entry_price = (position.entry_price * (position.size - order_result.quantity) + 
                           order_result.price * order_result.quantity) / position.size;
    position.update_price(51000.0);
    
    println!("   New position: {} {} at ${:.2} (current: ${:.2})", 
        position.size, position.symbol, position.entry_price, position.current_price);
    println!("   Unrealized PnL: ${:.2}", position.unrealized_pnl());

    // Create a funding payment
    println!("\n💰 Creating a funding payment...");
    let funding_payment = FundingPayment {
        timestamp: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
        position_size: position.size,
        funding_rate: 0.0001, // 0.01% funding rate
        payment_amount: 25.0, // $25 funding payment received
        mark_price: 51000.0,
    };
    
    println!("   Funding payment: ${:.2} for position size {} (rate: {:.4}%)", 
        funding_payment.payment_amount, funding_payment.position_size, 
        funding_payment.funding_rate * 100.0);

    // Apply funding payment to position
    position.apply_funding_payment(funding_payment.payment_amount);
    println!("   Position funding PnL: ${:.2}", position.funding_pnl);
    println!("   Total PnL: ${:.2}", position.total_pnl());

    // Generate risk orders
    println!("\n🛡️ Generating risk management orders...");
    if let Some(stop_loss) = risk_manager.generate_stop_loss(&position, "stop_123") {
        println!("   Stop loss order: {:?} {} at ${:.2}", 
            stop_loss.side, stop_loss.quantity, stop_loss.trigger_price);
    }
    
    if let Some(take_profit) = risk_manager.generate_take_profit(&position, "tp_123") {
        println!("   Take profit order: {:?} {} at ${:.2}", 
            take_profit.side, take_profit.quantity, take_profit.trigger_price);
    }

    println!("\n✅ Example completed successfully!");
    println!("\nThis example demonstrated:");
    println!("   - Creating positions and orders");
    println!("   - Using the risk manager");
    println!("   - Simulating order execution");
    println!("   - Handling funding payments");
    println!("   - Generating risk management orders");
    
    Ok(())
}