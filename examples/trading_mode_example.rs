use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use std::collections::HashMap;

use hyperliquid_backtester::unified_data_impl::{
    Position, OrderRequest, OrderResult, MarketData, Signal, SignalDirection,
    OrderSide, OrderType, TimeInForce, OrderStatus, TradingConfig, RiskConfig,
    SlippageConfig, ApiConfig, OrderBookLevel, OrderBookSnapshot, Trade
};

fn main() {
    println!("Hyperliquid Trading Mode Example");
    println!("================================");
    
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    
    // Create a position
    let mut position = Position::new("BTC", 1.0, 50000.0, 51000.0, now);
    println!("Position: {} {} @ ${}", 
        if position.is_long() { "LONG" } else { "SHORT" },
        position.size,
        position.entry_price
    );
    println!("Unrealized PnL: ${:.2}", position.unrealized_pnl);
    
    // Update position price
    position.update_price(52000.0);
    println!("Updated price: ${}", position.current_price);
    println!("New unrealized PnL: ${:.2}", position.unrealized_pnl);
    
    // Apply funding payment
    position.apply_funding_payment(100.0);
    println!("After funding payment:");
    println!("Funding PnL: ${:.2}", position.funding_pnl);
    println!("Total PnL: ${:.2}", position.total_pnl());
    
    // Create an order request
    let market_order = OrderRequest::market("BTC", OrderSide::Buy, 1.0);
    println!("\nMarket Order: {} {} {}", 
        market_order.side,
        market_order.quantity,
        market_order.symbol
    );
    
    let limit_order = OrderRequest::limit("ETH", OrderSide::Sell, 2.0, 3000.0)
        .reduce_only()
        .with_time_in_force(TimeInForce::FillOrKill);
    println!("Limit Order: {} {} {} @ ${} (reduce only: {})", 
        limit_order.side,
        limit_order.quantity,
        limit_order.symbol,
        limit_order.price.unwrap(),
        limit_order.reduce_only
    );
    
    // Create market data
    let market_data = MarketData::new(
        "BTC",
        50000.0,
        49990.0,
        50010.0,
        100.0,
        now,
    );
    println!("\nMarket Data for {}:", market_data.symbol);
    println!("Price: ${}", market_data.price);
    println!("Bid/Ask: ${}/{}", market_data.bid, market_data.ask);
    println!("Spread: ${} ({:.3}%)", 
        market_data.spread(), 
        market_data.spread_percentage()
    );
    
    // Create trading configuration
    let risk_config = RiskConfig {
        max_position_size_pct: 0.1,
        max_daily_loss_pct: 0.02,
        stop_loss_pct: 0.05,
        take_profit_pct: 0.1,
        max_leverage: 3.0,
        max_positions: 5,
        max_drawdown_pct: 0.2,
        use_trailing_stop: true,
        trailing_stop_distance_pct: Some(0.02),
    };
    
    let trading_config = TradingConfig {
        initial_balance: 10000.0,
        risk_config: Some(risk_config),
        slippage_config: None,
        api_config: None,
        parameters: HashMap::new(),
    };
    
    println!("\nTrading Configuration:");
    println!("Initial Balance: ${}", trading_config.initial_balance);
    println!("Max Position Size: {}%", 
        trading_config.risk_config.as_ref().unwrap().max_position_size_pct * 100.0
    );
    println!("Stop Loss: {}%", 
        trading_config.risk_config.as_ref().unwrap().stop_loss_pct * 100.0
    );
    
    println!("\nExample completed successfully!");
}