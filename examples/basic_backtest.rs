use chrono::{Duration, Utc};
use hyperliquid_backtester::prelude::*;
use rs_backtester::prelude::*;
use std::fs::File;
use std::io::Write;
use tracing::Instrument;

/// # Basic Backtest Example
///
/// This example demonstrates how to run a simple backtest using the Hyperliquid backtester.
/// It shows:
/// - Setting up logging for debugging and monitoring
/// - Fetching historical data from Hyperliquid API
/// - Creating a basic SMA crossover strategy
/// - Configuring backtest parameters with realistic commission rates
/// - Running the backtest with funding rates enabled
/// - Analyzing and reporting comprehensive results
/// - Comparing performance with and without funding rates
/// - Exporting results to CSV for further analysis
///
/// The example uses a 10/30 SMA crossover strategy on BTC/USD data over the last 90 days.
///
/// ## Usage
///
/// Run this example with:
/// ```bash
/// cargo run --example basic_backtest
/// ```
///
/// For debug logging:
/// ```bash
/// RUST_LOG=debug cargo run --example basic_backtest
/// ```
///
/// For JSON formatted logs:
/// ```bash
/// RUST_LOG=info HYPERLIQUID_LOG_FORMAT=json cargo run --example basic_backtest
/// ```

#[tokio::main]
async fn main() -> Result<(), HyperliquidBacktestError> {
    // Initialize logging for better debugging and monitoring
    init_logger_with_level("info");
    
    log::info!("Starting Hyperliquid Basic Backtest Example");
    
    println!("🚀 Hyperliquid Basic Backtest Example");
    println!("=====================================\n");

    // Fetch historical data for BTC with funding rates
    let end_time = Utc::now().timestamp() as u64;
    let start_time = end_time - (90 * 24 * 3600); // 90 days of data
    
    println!("Fetching BTC/USD data for the last 90 days...");
    let data = HyperliquidData::fetch_btc("1h", start_time, end_time).await?;
    
    println!("Data fetched: {} data points from {} to {}\n", 
        data.len(),
        data.datetime.first().unwrap().format("%Y-%m-%d %H:%M"),
        data.datetime.last().unwrap().format("%Y-%m-%d %H:%M"));
    
    // Create a simple SMA crossover strategy
    println!("Setting up SMA crossover strategy (10/30)...");
    let mut strategy = Strategy::new();
    
    // Strategy parameters
    let short_period = 10;
    let long_period = 30;
    
    // Initialize strategy
    strategy.init(Box::new(move |_ctx, _data| {
        // No initialization needed for this simple strategy
    }));
    
    // Define strategy logic
    strategy.next(Box::new(move |ctx, data| {
        // Wait until we have enough data for the long SMA
        if data.index < long_period {
            return;
        }
        
        // Calculate short and long SMAs
        let mut short_sum = 0.0;
        let mut long_sum = 0.0;
        
        for i in 0..short_period {
            short_sum += data.close[data.index - i];
        }
        
        for i in 0..long_period {
            long_sum += data.close[data.index - i];
        }
        
        let short_sma = short_sum / short_period as f64;
        let long_sma = long_sum / long_period as f64;
        
        // Get current position
        let position = ctx.position();
        
        // Trading logic
        if short_sma > long_sma && position <= 0.0 {
            // Bullish crossover - go long
            ctx.entry_qty(1.0);
        } else if short_sma < long_sma && position >= 0.0 {
            // Bearish crossover - go short
            ctx.entry_qty(-1.0);
        }
    }));
    
    // Set up backtest parameters
    let initial_capital = 10000.0; // $10,000
    
    // Create commission with funding enabled
    let commission = HyperliquidCommission {
        maker_rate: 0.0002,  // 0.02% maker fee
        taker_rate: 0.0005,  // 0.05% taker fee
        funding_enabled: true,
    };
    
    println!("Running backtest with funding rates enabled...");
    
    // Create and run backtest
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy,
        initial_capital,
        commission,
    );
    
    // Run backtest with funding rates
    backtest.calculate_with_funding();
    
    // Get backtest results
    let stats = backtest.stats();
    
    // Print backtest results
    println!("\nBacktest Results:");
    println!("----------------");
    println!("Initial Capital: ${:.2}", initial_capital);
    println!("Final Capital: ${:.2}", stats.final_capital);
    println!("Net Profit: ${:.2} ({:.2}%)", 
        stats.net_profit,
        stats.net_profit_pct * 100.0);
    println!("Max Drawdown: {:.2}%", stats.max_drawdown * 100.0);
    println!("Win Rate: {:.2}%", stats.win_rate * 100.0);
    println!("Profit Factor: {:.2}", stats.profit_factor);
    println!("Sharpe Ratio: {:.2}", stats.sharpe_ratio);
    
    // Get funding impact
    let funding_impact = backtest.funding_impact();
    println!("\nFunding Rate Impact:");
    println!("------------------");
    println!("Total Funding Payments: ${:.2}", funding_impact.total_funding);
    println!("Funding as % of PnL: {:.2}%", 
        if stats.net_profit != 0.0 {
            (funding_impact.total_funding / stats.net_profit).abs() * 100.0
        } else {
            0.0
        });
    
    // Get trade statistics
    let trade_stats = backtest.trade_stats();
    println!("\nTrade Statistics:");
    println!("----------------");
    println!("Total Trades: {}", trade_stats.total_trades);
    println!("Winning Trades: {}", trade_stats.winning_trades);
    println!("Losing Trades: {}", trade_stats.losing_trades);
    println!("Average Profit per Trade: ${:.2}", trade_stats.avg_profit_per_trade);
    println!("Average Profit per Winning Trade: ${:.2}", trade_stats.avg_profit_per_winning_trade);
    println!("Average Loss per Losing Trade: ${:.2}", trade_stats.avg_loss_per_losing_trade);
    
    // Export results to CSV
    println!("\nExporting results to CSV...");
    let csv_data = backtest.to_csv()?;
    let csv_file = "basic_backtest_results.csv";
    let mut file = File::create(csv_file)?;
    file.write_all(csv_data.as_bytes())?;
    println!("Results exported to {}", csv_file);
    
    // Run the same backtest without funding to compare
    println!("\nRunning comparison backtest without funding rates...");
    let mut commission_no_funding = commission.clone();
    commission_no_funding.funding_enabled = false;
    
    let mut backtest_no_funding = HyperliquidBacktest::new(
        data.clone(),
        Strategy::new_from_fn(move |ctx, data| {
            // Same strategy logic as above
            if data.index < long_period {
                return;
            }
            
            let mut short_sum = 0.0;
            let mut long_sum = 0.0;
            
            for i in 0..short_period {
                short_sum += data.close[data.index - i];
            }
            
            for i in 0..long_period {
                long_sum += data.close[data.index - i];
            }
            
            let short_sma = short_sum / short_period as f64;
            let long_sma = long_sum / long_period as f64;
            
            let position = ctx.position();
            
            if short_sma > long_sma && position <= 0.0 {
                ctx.entry_qty(1.0);
            } else if short_sma < long_sma && position >= 0.0 {
                ctx.entry_qty(-1.0);
            }
        }),
        initial_capital,
        commission_no_funding,
    );
    
    backtest_no_funding.calculate();
    
    let stats_no_funding = backtest_no_funding.stats();
    
    println!("\nComparison Results (Without Funding):");
    println!("-----------------------------------");
    println!("Net Profit: ${:.2} ({:.2}%)", 
        stats_no_funding.net_profit,
        stats_no_funding.net_profit_pct * 100.0);
    
    println!("\nFunding Impact on Performance:");
    println!("----------------------------");
    println!("Net Profit Difference: ${:.2}", 
        stats.net_profit - stats_no_funding.net_profit);
    println!("Performance Impact: {:.2}%", 
        ((stats.net_profit / stats_no_funding.net_profit) - 1.0) * 100.0);
    
    println!("\nExample completed successfully!");
    
    Ok(())
}