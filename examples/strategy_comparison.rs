use chrono::{Duration, Utc};
use hyperliquid_backtest::prelude::*;
use hyperliquid_backtest::csv_export::*;
use rs_backtester::prelude::*;
use std::fs::File;
use std::io::Write;

/// # Strategy Comparison Example
///
/// This example demonstrates how to compare multiple trading strategies
/// using the Hyperliquid backtester.

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hyperliquid Strategy Comparison Example");
    println!("=====================================\n");

    // Fetch historical data for BTC with funding rates
    let end_time = Utc::now().timestamp() as u64;
    let start_time = end_time - (120 * 24 * 3600); // 120 days of data
    
    println!("Fetching BTC/USD data for the last 120 days...");
    let data = HyperliquidData::fetch_btc("1h", start_time, end_time).await?;
    
    println!("Data fetched: {} data points", data.len());
    
    // Set up backtest parameters
    let initial_capital = 10000.0; // $10,000
    let commission = HyperliquidCommission {
        maker_rate: 0.0002,  // 0.02% maker fee
        taker_rate: 0.0005,  // 0.05% taker fee
        funding_enabled: true,
    };
    
    println!("Setting up strategies for comparison...");
    
    // Strategy 1: Standard SMA Crossover
    println!("Strategy 1: Standard SMA Crossover (10/30)");
    let mut sma_strategy = Strategy::new();
    
    // Strategy parameters
    let short_period = 10;
    let long_period = 30;
    
    sma_strategy.next(Box::new(move |ctx, data| {
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
    
    // Strategy 2: Enhanced SMA with Funding Awareness
    println!("Strategy 2: Enhanced SMA with Funding Awareness");
    let enhanced_sma = enhanced_sma_cross(
        short_period,
        long_period,
        FundingAwareConfig {
            funding_threshold: 0.0005,  // 0.05% threshold
            position_adjustment: 0.5,   // Reduce position size by 50% when funding is unfavorable
            enable_counter_trading: true, // Allow counter-trend trading on extreme funding
        }
    );
    
    // Strategy 3: Funding Arbitrage Strategy
    println!("Strategy 3: Pure Funding Arbitrage");
    let funding_arb = funding_arbitrage_strategy(
        0.0008,  // 0.08% threshold for taking positions
        24,      // 24-hour lookback period
        0.5      // 50% position size
    );
    
    // Run backtests
    println!("\nRunning backtests...");
    
    // Backtest Strategy 1: Standard SMA
    let mut backtest_sma = HyperliquidBacktest::new(
        data.clone(),
        sma_strategy,
        initial_capital,
        commission.clone(),
    );
    
    backtest_sma.calculate_with_funding();
    let stats_sma = backtest_sma.stats();
    
    // Backtest Strategy 2: Enhanced SMA
    let mut backtest_enhanced = HyperliquidBacktest::new(
        data.clone(),
        enhanced_sma,
        initial_capital,
        commission.clone(),
    );
    
    backtest_enhanced.calculate_with_funding();
    let stats_enhanced = backtest_enhanced.stats();
    
    // Backtest Strategy 3: Funding Arbitrage
    let mut backtest_arb = HyperliquidBacktest::new(
        data.clone(),
        funding_arb,
        initial_capital,
        commission.clone(),
    );
    
    backtest_arb.calculate_with_funding();
    let stats_arb = backtest_arb.stats();
    
    // Create strategy comparison data
    println!("\nGenerating strategy comparison data...");
    
    let comparison_data = StrategyComparisonData::new(vec![
        ("Standard SMA", backtest_sma),
        ("Enhanced SMA", backtest_enhanced),
        ("Funding Arbitrage", backtest_arb),
    ]);
    
    // Export comparison to CSV
    let comparison_csv = comparison_data.to_csv()?;
    let comparison_file = "strategy_comparison.csv";
    let mut file = File::create(comparison_file)?;
    file.write_all(comparison_csv.as_bytes())?;
    println!("Comparison data exported to {}", comparison_file);
    
    // Export equity curves
    let equity_curves = comparison_data.equity_curves_csv()?;
    let equity_file = "equity_curves.csv";
    let mut file = File::create(equity_file)?;
    file.write_all(equity_curves.as_bytes())?;
    println!("Equity curves exported to {}", equity_file);
    
    // Generate performance summary table
    println!("\nPerformance Summary:");
    println!("-------------------");
    println!("{:<20} {:<15} {:<15} {:<15}", 
        "Strategy", "Net Profit", "Max DD", "Sharpe");
    println!("{:<20} ${:<14.2} {:<14.2}% {:<14.2}", 
        "Standard SMA", 
        stats_sma.net_profit, 
        stats_sma.max_drawdown * 100.0,
        stats_sma.sharpe_ratio);
    println!("{:<20} ${:<14.2} {:<14.2}% {:<14.2}", 
        "Enhanced SMA", 
        stats_enhanced.net_profit, 
        stats_enhanced.max_drawdown * 100.0,
        stats_enhanced.sharpe_ratio);
    println!("{:<20} ${:<14.2} {:<14.2}% {:<14.2}", 
        "Funding Arbitrage", 
        stats_arb.net_profit, 
        stats_arb.max_drawdown * 100.0,
        stats_arb.sharpe_ratio);
    
    println!("\nExample completed successfully!");
    
    Ok(())
}