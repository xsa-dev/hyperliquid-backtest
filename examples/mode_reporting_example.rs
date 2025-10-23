use std::collections::HashMap;

use chrono::{FixedOffset, Utc};
use hyperliquid_backtest::risk_manager::{RiskConfig, RiskManager};
use hyperliquid_backtest::unified_data::{OrderRequest, OrderSide, Position, TimeInForce};

fn main() {
    println!("Mode reporting placeholder example\n==============================\n");

    let tz = FixedOffset::east_opt(0).expect("valid timezone offset");
    let timestamp = Utc::now().with_timezone(&tz);

    // Track an existing position so that risk validation has some context.
    let mut positions = HashMap::new();
    let mut btc_position = Position::new("BTC-PERP", 0.5, 50_000.0, 50_150.0, timestamp);
    btc_position.realized_pnl = 125.0;
    btc_position.apply_funding_payment(12.5);
    positions.insert(btc_position.symbol.clone(), btc_position.clone());

    // Configure a lightweight risk manager that keeps position sizes below 5% of equity
    // while attaching basic stop-loss and take-profit orders.
    let mut risk_manager = RiskManager::new(
        RiskConfig {
            max_position_size_pct: 0.05,
            stop_loss_pct: 0.02,
            take_profit_pct: 0.04,
        },
        100_000.0,
    );

    // Build a limit order request using the simplified unified data structures.
    let mut entry = OrderRequest::limit("BTC-PERP", OrderSide::Buy, 0.05, 50_000.0);
    entry.time_in_force = TimeInForce::ImmediateOrCancel;
    entry.client_order_id = Some("demo-entry".into());

    risk_manager
        .validate_order(&entry, &positions)
        .expect("order should pass risk checks");

    println!(
        "Validated {:?} order for {:?} {} contracts @ {:?}",
        entry.order_type, entry.side, entry.quantity, entry.price
    );

    // Demonstrate how stop-loss and take-profit orders are generated and tracked.
    if let Some(stop_loss) = risk_manager.generate_stop_loss(&btc_position, "order-1") {
        risk_manager.register_stop_loss(stop_loss.clone());
        println!(
            "Registered stop-loss: {:?} {} @ {:.2}",
            stop_loss.side, stop_loss.quantity, stop_loss.trigger_price
        );
    }

    if let Some(take_profit) = risk_manager.generate_take_profit(&btc_position, "order-1") {
        risk_manager.register_take_profit(take_profit.clone());
        println!(
            "Registered take-profit: {:?} {} @ {:.2}",
            take_profit.side, take_profit.quantity, take_profit.trigger_price
        );
    }

    // Price data arrives and the risk manager checks whether any orders should fire.
    let mut latest_prices = HashMap::new();
    latest_prices.insert("BTC-PERP".to_string(), 48_750.0);
    let triggered = risk_manager.check_risk_orders(&latest_prices);

    for order in triggered {
        println!(
            "Triggered {:?} {:?} order for {} at {:.2}",
            order.order_type, order.side, order.symbol, order.trigger_price
        );
    }
}
