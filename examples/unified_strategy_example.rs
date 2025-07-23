use std::collections::HashMap;
use chrono::{DateTime, FixedOffset, Utc};
use tokio::time::{sleep, Duration};

use hyperliquid_backtester::prelude::*;
use hyperliquid_backtester::strategies::{
    TradingStrategy, FundingAwareConfig,
    create_funding_arbitrage_strategy, create_enhanced_sma_strategy
};
use hyperliquid_backtester::unified_data_impl::{
    MarketData, OrderRequest, OrderResult, FundingPayment,
    OrderSide, OrderType, OrderStatus, TimeInForce
};
use hyperliquid_backtester::trading_mode::{TradingMode, TradingModeManager};

// Example function to demonstrate using strategies in backtest mode
async fn run_backtest_example() -> Result<(), HyperliquidBacktestError> {
    println!("Running backtest example...");
    
    // Fetch historical data
    let start_time = chrono::Utc::now() - chrono::Duration::days(30);
    let end_time = chrono::Utc::now();
    
    let data = HyperliquidData::fetch(
        "BTC",
        "1h",
        start_time.timestamp() as u64,
        end_time.timestamp() as u64
    ).await?;
    
    println!("Fetched {} data points for BTC", data.datetime.len());
    
    // Create funding arbitrage strategy
    let funding_strategy = create_funding_arbitrage_strategy(0.0002)
        .map_err(|e| HyperliquidBacktestError::StrategyError(e))?;
    
    // Create enhanced SMA strategy
    let funding_config = FundingAwareConfig {
        funding_threshold: 0.0001,
        funding_weight: 0.5,
        use_funding_direction: true,
        use_funding_prediction: false,
    };
    
    let sma_strategy = create_enhanced_sma_strategy(12, 26, Some(funding_config))
        .map_err(|e| HyperliquidBacktestError::StrategyError(e))?;
    
    // Run backtest with funding arbitrage strategy
    println!("\nRunning backtest with funding arbitrage strategy...");
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        Box::new(BacktestStrategyAdapter::new(funding_strategy)),
        10000.0,
        HyperliquidCommission::default(),
    )?;
    
    backtest.calculate_with_funding()?;
    let report = backtest.funding_report()?;
    
    println!("Funding arbitrage strategy results:");
    println!("  Net profit: ${:.2}", report.net_profit);
    println!("  Return: {:.2}%", report.net_profit / 10000.0 * 100.0);
    println!("  Funding PnL: ${:.2}", report.net_funding_pnl);
    println!("  Trading PnL: ${:.2}", report.net_trading_pnl);
    
    // Run backtest with enhanced SMA strategy
    println!("\nRunning backtest with enhanced SMA strategy...");
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        Box::new(BacktestStrategyAdapter::new(sma_strategy)),
        10000.0,
        HyperliquidCommission::default(),
    )?;
    
    backtest.calculate_with_funding()?;
    let report = backtest.funding_report()?;
    
    println!("Enhanced SMA strategy results:");
    println!("  Net profit: ${:.2}", report.net_profit);
    println!("  Return: {:.2}%", report.net_profit / 10000.0 * 100.0);
    println!("  Funding PnL: ${:.2}", report.net_funding_pnl);
    println!("  Trading PnL: ${:.2}", report.net_trading_pnl);
    
    Ok(())
}

// Example function to demonstrate using strategies in paper trading mode
async fn run_paper_trading_example() -> Result<(), HyperliquidBacktestError> {
    println!("\nRunning paper trading example...");
    
    // Create funding arbitrage strategy
    let strategy = create_funding_arbitrage_strategy(0.0002)
        .map_err(|e| HyperliquidBacktestError::StrategyError(e))?;
    
    // Initialize paper trading engine
    let mut paper_engine = PaperTradingEngine::new(10000.0, Default::default())?;
    
    // Connect to real-time data stream
    let mut data_stream = RealTimeDataStream::new().await?;
    data_stream.subscribe_to_ticker("BTC").await?;
    
    // Simulate paper trading for a few iterations
    println!("Simulating paper trading for 5 iterations...");
    for i in 1..=5 {
        // Simulate receiving market data
        let timestamp = Utc::now().with_timezone(&FixedOffset::east(0));
        let price = 50000.0 + (i as f64 * 100.0);
        
        let market_data = MarketData::new(
            "BTC",
            price,
            price - 10.0,
            price + 10.0,
            1000.0,
            timestamp,
        ).with_funding_rate(
            0.0002,
            timestamp + chrono::Duration::hours(8)
        );
        
        // Process market data with strategy
        let orders = strategy.on_market_data(&market_data)
            .map_err(|e| HyperliquidBacktestError::StrategyError(e))?;
        
        // Execute orders in paper trading engine
        for order in orders {
            let result = paper_engine.execute_order(order).await?;
            
            if result.status == OrderStatus::Filled || result.status == OrderStatus::PartiallyFilled {
                // Notify strategy of fill
                strategy.on_order_fill(&result)
                    .map_err(|e| HyperliquidBacktestError::StrategyError(e))?;
                
                println!("  Order executed: {} {} {} at {} with size {}",
                    result.symbol,
                    result.side,
                    result.order_type,
                    result.average_price.unwrap_or(0.0),
                    result.filled_quantity
                );
            }
        }
        
        // Get current positions and portfolio value
        let positions = paper_engine.get_current_positions();
        let portfolio_value = paper_engine.get_portfolio_value();
        
        println!("  Iteration {}: Portfolio value: ${:.2}", i, portfolio_value);
        for (symbol, position) in positions {
            println!("    Position: {} {} @ ${:.2} (PnL: ${:.2})",
                symbol,
                position.size,
                position.entry_price,
                position.unrealized_pnl
            );
        }
        
        // Simulate funding payment every 3rd iteration
        if i % 3 == 0 {
            for (symbol, position) in paper_engine.get_current_positions() {
                if position.size != 0.0 {
                    let funding_payment = FundingPayment {
                        symbol: symbol.clone(),
                        rate: 0.0002,
                        position_size: position.size,
                        amount: position.size * 0.0002 * position.current_price,
                        timestamp,
                    };
                    
                    // Apply funding payment
                    paper_engine.apply_funding_payment(&funding_payment)?;
                    
                    // Notify strategy of funding payment
                    strategy.on_funding_payment(&funding_payment)
                        .map_err(|e| HyperliquidBacktestError::StrategyError(e))?;
                    
                    println!("  Funding payment applied: {} rate={}, amount=${:.2}",
                        funding_payment.symbol,
                        funding_payment.rate,
                        funding_payment.amount
                    );
                }
            }
        }
        
        // Simulate delay between iterations
        sleep(Duration::from_millis(500)).await;
    }
    
    // Get final portfolio value and performance metrics
    let portfolio_value = paper_engine.get_portfolio_value();
    let performance = paper_engine.get_performance_metrics();
    
    println!("\nPaper trading results:");
    println!("  Final portfolio value: ${:.2}", portfolio_value);
    println!("  Return: {:.2}%", (portfolio_value / 10000.0 - 1.0) * 100.0);
    println!("  Trading PnL: ${:.2}", performance.trading_pnl);
    println!("  Funding PnL: ${:.2}", performance.funding_pnl);
    
    Ok(())
}

// Example function to demonstrate using strategies in live trading mode (simulated)
async fn run_live_trading_simulation() -> Result<(), HyperliquidBacktestError> {
    println!("\nRunning live trading simulation...");
    println!("Note: This is a simulation and does not execute real trades");
    
    // Create enhanced SMA strategy
    let funding_config = FundingAwareConfig {
        funding_threshold: 0.0001,
        funding_weight: 0.3,
        use_funding_direction: true,
        use_funding_prediction: false,
    };
    
    let strategy = create_enhanced_sma_strategy(12, 26, Some(funding_config))
        .map_err(|e| HyperliquidBacktestError::StrategyError(e))?;
    
    // Initialize simulated live trading engine
    let mut live_engine = SimulatedLiveTradingEngine::new(10000.0)?;
    
    // Simulate live trading for a few iterations
    println!("Simulating live trading for 5 iterations...");
    for i in 1..=5 {
        // Simulate receiving market data
        let timestamp = Utc::now().with_timezone(&FixedOffset::east(0));
        let price = 3000.0 + (i as f64 * 10.0);
        
        let market_data = MarketData::new(
            "ETH",
            price,
            price - 1.0,
            price + 1.0,
            500.0,
            timestamp,
        ).with_funding_rate(
            -0.0001,
            timestamp + chrono::Duration::hours(8)
        );
        
        // Process market data with strategy
        let orders = strategy.on_market_data(&market_data)
            .map_err(|e| HyperliquidBacktestError::StrategyError(e))?;
        
        // Execute orders in simulated live trading engine
        for order in orders {
            let result = live_engine.execute_order(order).await?;
            
            if result.status == OrderStatus::Filled || result.status == OrderStatus::PartiallyFilled {
                // Notify strategy of fill
                strategy.on_order_fill(&result)
                    .map_err(|e| HyperliquidBacktestError::StrategyError(e))?;
                
                println!("  Order executed: {} {} {} at {} with size {}",
                    result.symbol,
                    result.side,
                    result.order_type,
                    result.average_price.unwrap_or(0.0),
                    result.filled_quantity
                );
            }
        }
        
        // Get current positions and account value
        let positions = live_engine.get_current_positions().await?;
        let account_value = live_engine.get_account_value().await?;
        
        println!("  Iteration {}: Account value: ${:.2}", i, account_value);
        for (symbol, position) in positions {
            println!("    Position: {} {} @ ${:.2} (PnL: ${:.2})",
                symbol,
                position.size,
                position.entry_price,
                position.unrealized_pnl
            );
        }
        
        // Simulate delay between iterations
        sleep(Duration::from_millis(500)).await;
    }
    
    // Get final account value and performance metrics
    let account_value = live_engine.get_account_value().await?;
    let performance = live_engine.get_performance_metrics().await?;
    
    println!("\nLive trading simulation results:");
    println!("  Final account value: ${:.2}", account_value);
    println!("  Return: {:.2}%", (account_value / 10000.0 - 1.0) * 100.0);
    println!("  Trading PnL: ${:.2}", performance.trading_pnl);
    println!("  Funding PnL: ${:.2}", performance.funding_pnl);
    
    Ok(())
}

// Adapter to use TradingStrategy with rs-backtester
struct BacktestStrategyAdapter {
    strategy: Box<dyn TradingStrategy>,
    current_index: usize,
}

impl BacktestStrategyAdapter {
    fn new(strategy: Box<dyn TradingStrategy>) -> Self {
        Self {
            strategy,
            current_index: 0,
        }
    }
}

impl rs_backtester::strategies::Strategy for BacktestStrategyAdapter {
    fn next(&mut self, ctx: &mut rs_backtester::strategies::Context, _: &mut rs_backtester::strategies::Broker) {
        let index = ctx.index();
        self.current_index = index;
        
        // Convert rs-backtester data to MarketData
        let data = ctx.data();
        let timestamp = data.datetime[index];
        
        let market_data = MarketData::new(
            &data.ticker,
            data.close[index],
            data.low[index],
            data.high[index],
            data.volume[index],
            timestamp,
        );
        
        // Process market data with strategy
        if let Ok(orders) = self.strategy.on_market_data(&market_data) {
            // Convert orders to rs-backtester actions
            for order in orders {
                match order.side {
                    OrderSide::Buy => {
                        ctx.entry_qty(order.quantity);
                    },
                    OrderSide::Sell => {
                        ctx.entry_qty(-order.quantity);
                    },
                }
            }
        }
    }
}

// Simulated live trading engine for example purposes
struct SimulatedLiveTradingEngine {
    balance: f64,
    positions: HashMap<String, Position>,
    order_history: Vec<OrderResult>,
    next_order_id: usize,
}

impl SimulatedLiveTradingEngine {
    fn new(initial_balance: f64) -> Result<Self, HyperliquidBacktestError> {
        Ok(Self {
            balance: initial_balance,
            positions: HashMap::new(),
            order_history: Vec::new(),
            next_order_id: 1,
        })
    }
    
    async fn execute_order(&mut self, order: OrderRequest) -> Result<OrderResult, HyperliquidBacktestError> {
        // Simulate order execution
        let timestamp = Utc::now().with_timezone(&FixedOffset::east(0));
        let order_id = format!("order_{}", self.next_order_id);
        self.next_order_id += 1;
        
        // Simulate execution price (use requested price or add/subtract small amount)
        let execution_price = match order.price {
            Some(price) => price,
            None => {
                // For market orders, simulate some slippage
                let position = self.positions.get(&order.symbol).cloned();
                match order.side {
                    OrderSide::Buy => {
                        if let Some(pos) = &position {
                            pos.current_price * 1.001 // 0.1% slippage for buys
                        } else {
                            3000.0 // Default price for ETH
                        }
                    },
                    OrderSide::Sell => {
                        if let Some(pos) = &position {
                            pos.current_price * 0.999 // 0.1% slippage for sells
                        } else {
                            3000.0 // Default price for ETH
                        }
                    },
                }
            }
        };
        
        // Calculate fees (0.05% taker fee)
        let fees = execution_price * order.quantity * 0.0005;
        
        // Create order result
        let mut result = OrderResult {
            order_id,
            symbol: order.symbol.clone(),
            side: order.side,
            order_type: order.order_type,
            requested_quantity: order.quantity,
            filled_quantity: order.quantity,
            average_price: Some(execution_price),
            status: OrderStatus::Filled,
            timestamp,
            fees: Some(fees),
            error: None,
            client_order_id: order.client_order_id.clone(),
            metadata: HashMap::new(),
        };
        
        // Update position
        let position_size = match order.side {
            OrderSide::Buy => order.quantity,
            OrderSide::Sell => -order.quantity,
        };
        
        let position = self.positions.entry(order.symbol.clone()).or_insert_with(|| {
            Position::new(
                &order.symbol,
                0.0,
                execution_price,
                execution_price,
                timestamp,
            )
        });
        
        // Update position
        if position.size == 0.0 {
            // New position
            position.size = position_size;
            position.entry_price = execution_price;
        } else if position.size > 0.0 && position_size > 0.0 {
            // Adding to long position
            let new_size = position.size + position_size;
            position.entry_price = (position.entry_price * position.size + execution_price * position_size) / new_size;
            position.size = new_size;
        } else if position.size < 0.0 && position_size < 0.0 {
            // Adding to short position
            let new_size = position.size + position_size;
            position.entry_price = (position.entry_price * position.size + execution_price * position_size) / new_size;
            position.size = new_size;
        } else {
            // Reducing or flipping position
            let new_size = position.size + position_size;
            if new_size.abs() < 0.000001 {
                // Position closed
                position.realized_pnl += position.unrealized_pnl;
                position.size = 0.0;
                position.unrealized_pnl = 0.0;
            } else if new_size * position.size < 0.0 {
                // Position flipped
                let closed_size = position.size;
                let closed_pnl = closed_size * (execution_price - position.entry_price);
                position.realized_pnl += closed_pnl;
                position.size = new_size;
                position.entry_price = execution_price;
                position.unrealized_pnl = 0.0;
            } else {
                // Position reduced
                let closed_size = -position_size;
                let closed_pnl = closed_size * (execution_price - position.entry_price);
                position.realized_pnl += closed_pnl;
                position.size = new_size;
                position.unrealized_pnl = position.size * (execution_price - position.entry_price);
            }
        }
        
        // Update current price
        position.current_price = execution_price;
        position.timestamp = timestamp;
        
        // Update balance (subtract fees)
        self.balance -= fees;
        
        // Add to order history
        self.order_history.push(result.clone());
        
        Ok(result)
    }
    
    async fn get_current_positions(&self) -> Result<HashMap<String, Position>, HyperliquidBacktestError> {
        Ok(self.positions.clone())
    }
    
    async fn get_account_value(&self) -> Result<f64, HyperliquidBacktestError> {
        let mut total = self.balance;
        
        // Add unrealized PnL from positions
        for (_, position) in &self.positions {
            total += position.unrealized_pnl;
        }
        
        Ok(total)
    }
    
    async fn get_performance_metrics(&self) -> Result<PerformanceMetrics, HyperliquidBacktestError> {
        let mut trading_pnl = 0.0;
        let mut funding_pnl = 0.0;
        
        // Sum up realized PnL and funding PnL
        for (_, position) in &self.positions {
            trading_pnl += position.realized_pnl + position.unrealized_pnl;
            funding_pnl += position.funding_pnl;
        }
        
        Ok(PerformanceMetrics {
            trading_pnl,
            funding_pnl,
            total_pnl: trading_pnl + funding_pnl,
            total_fees: self.order_history.iter()
                .filter_map(|order| order.fees)
                .sum(),
        })
    }
}

// Simple performance metrics structure
struct PerformanceMetrics {
    trading_pnl: f64,
    funding_pnl: f64,
    total_pnl: f64,
    total_fees: f64,
}

#[tokio::main]
async fn main() -> Result<(), HyperliquidBacktestError> {
    println!("Unified Strategy Framework Example");
    println!("==================================");
    
    // Run backtest example
    run_backtest_example().await?;
    
    // Run paper trading example
    run_paper_trading_example().await?;
    
    // Run live trading simulation
    run_live_trading_simulation().await?;
    
    println!("\nExample completed successfully!");
    Ok(())
}