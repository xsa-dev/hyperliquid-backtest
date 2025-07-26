use chrono::{DateTime, Duration, FixedOffset, Utc};
use hyperliquid_backtest::prelude::*;
use hyperliquid_backtest::funding_report::*;
use std::fs::File;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hyperliquid Funding Report Example");
    println!("=================================\n");

    // Fetch historical data for BTC with funding rates
    let end_time = Utc::now().timestamp() as u64;
    let start_time = end_time - (30 * 24 * 3600); // 30 days of data
    
    println!("Fetching BTC/USD data for the last 30 days...");
    let data = HyperliquidData::fetch_btc("1h", start_time, end_time).await?;
    
    println!("Data fetched: {} data points from {} to {}\n", 
        data.len(),
        data.datetime.first().unwrap().format("%Y-%m-%d %H:%M"),
        data.datetime.last().unwrap().format("%Y-%m-%d %H:%M"));
    
    // Create a simple strategy for testing
    println!("Running backtest with SMA crossover strategy...");
    let mut strategy = Strategy::new();
    
    // Simple SMA crossover strategy
    let short_period = 10;
    let long_period = 30;
    
    strategy.init(Box::new(move |_ctx, _data| {
        // Initialize strategy
    }));
    
    strategy.next(Box::new(move |ctx, data| {
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
    
    // Convert to rs-backtester Data format
    let rs_data = data.to_rs_backtester_data();
    
    // Create commission with funding enabled
    let commission = HyperliquidCommission {
        maker_rate: 0.0002,  // 0.02% maker fee
        taker_rate: 0.0005,  // 0.05% taker fee
        funding_enabled: true,
    };
    
    // Run backtest
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy,
        10000.0, // $10,000 initial capital
        commission,
    );
    
    backtest.calculate_with_funding();
    
    // Generate funding report
    println!("Generating funding report...");
    let funding_report = backtest.funding_report()?;
    
    // Print summary
    println!("\nFunding Report Summary:");
    println!("{}", funding_report.summary());
    
    // Generate detailed report
    let detailed_report = funding_report.detailed_report()?;
    
    // Save detailed report to file
    let report_file = "funding_report.md";
    let mut file = File::create(report_file)?;
    file.write_all(detailed_report.as_bytes())?;
    println!("\nDetailed report saved to {}", report_file);
    
    // Export funding metrics by period
    let metrics_csv = funding_report.export_metrics_by_period_to_csv()?;
    let metrics_file = "funding_metrics.csv";
    let mut file = File::create(metrics_file)?;
    file.write_all(metrics_csv.as_bytes())?;
    println!("Funding metrics by period saved to {}", metrics_file);
    
    // Export funding payment data
    let payments_csv = funding_report.to_csv()?;
    let payments_file = "funding_payments.csv";
    let mut file = File::create(payments_file)?;
    file.write_all(payments_csv.as_bytes())?;
    println!("Funding payments data saved to {}", payments_file);
    
    // Generate visualization data
    let viz_data = funding_report.visualization_data()?;
    println!("\nVisualization data generated with {} funding rate points", viz_data.rates.len());
    
    // Calculate funding PnL breakdown
    let pnl_breakdown = funding_report.calculate_funding_pnl_breakdown();
    
    println!("\nFunding PnL Breakdown:");
    println!("Long positions: ${:.2} ({}%)", 
        pnl_breakdown.long_net, 
        (pnl_breakdown.long_percentage * 100.0).round());
    println!("Short positions: ${:.2} ({}%)", 
        pnl_breakdown.short_net, 
        (pnl_breakdown.short_percentage * 100.0).round());
    println!("Total: ${:.2}", pnl_breakdown.total_net);
    
    // Try to analyze funding regimes
    match funding_report.analyze_funding_regimes() {
        Ok(regime_analysis) => {
            println!("\nFunding Regime Analysis:");
            println!("Total regimes detected: {}", regime_analysis.total_regimes);
            println!("Positive regimes: {}", regime_analysis.positive_regimes);
            println!("Negative regimes: {}", regime_analysis.negative_regimes);
            println!("Neutral regimes: {}", regime_analysis.neutral_regimes);
            println!("Average regime duration: {:.1} periods", regime_analysis.avg_regime_duration);
        },
        Err(e) => {
            println!("\nCould not analyze funding regimes: {}", e);
        }
    }
    
    println!("\nExample completed successfully!");
    
    Ok(())
}