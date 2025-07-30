use chrono::{DateTime, Duration, FixedOffset, Utc};
use hyperliquid_backtest::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hyperliquid Enhanced CSV Export Example");
    println!("======================================\n");

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
    
    // Create output directory if it doesn't exist
    let output_dir = "csv_exports";
    if !Path::new(output_dir).exists() {
        std::fs::create_dir_all(output_dir)?;
    }
    
    // Export using the EnhancedCsvExportExt trait
    println!("\nExporting CSV files using EnhancedCsvExportExt trait...");
    
    // Export backtest results with funding data
    let backtest_file = format!("{}/backtest_results.csv", output_dir);
    backtest.export_to_csv(&backtest_file)?;
    println!("Backtest results exported to {}", backtest_file);
    
    // Export funding rate history with enhanced data
    let funding_history_file = format!("{}/funding_history.csv", output_dir);
    backtest.export_funding_history_to_csv(&funding_history_file)?;
    println!("Enhanced funding rate history exported to {}", funding_history_file);
    
    // Export funding payments
    let funding_payments_file = format!("{}/funding_payments.csv", output_dir);
    backtest.export_funding_payments_to_csv(&funding_payments_file)?;
    println!("Funding payments exported to {}", funding_payments_file);
    
    // Export detailed funding statistics
    let funding_stats_file = format!("{}/funding_statistics.csv", output_dir);
    backtest.export_funding_statistics_to_csv(&funding_stats_file)?;
    println!("Detailed funding statistics exported to {}", funding_stats_file);
    
    // Export all funding data at once
    println!("\nExporting all funding data at once...");
    let base_path = format!("{}/all_data", output_dir);
    let exported_files = backtest.export_all_funding_data(&base_path)?;
    
    println!("Exported {} files:", exported_files.len());
    for file in exported_files {
        println!("- {}", file);
    }
    
    // Demonstrate how to use the exported data
    println!("\nUsage examples for exported data:");
    println!("1. The backtest_results.csv file contains OHLC data with position sizes and PnL");
    println!("   - Use this for general backtest analysis and visualization");
    println!("2. The funding_history.csv file contains detailed funding rate data");
    println!("   - Use this for funding rate trend analysis and correlation studies");
    println!("3. The funding_statistics.csv file contains aggregated funding metrics");
    println!("   - Use this for quick assessment of funding impact on strategy performance");
    println!("4. The funding_payments.csv file shows individual funding payments");
    println!("   - Use this to analyze funding cash flows over time");
    println!("5. The funding_metrics.csv file provides period-based funding analysis");
    println!("   - Use this for seasonal funding patterns (daily, weekly, monthly)");
    println!("6. The funding_histogram.csv file shows the distribution of funding rates");
    println!("   - Use this to understand funding rate volatility and patterns");
    
    // Demonstrate using EnhancedCsvExport directly
    println!("\nCreating custom CSV export...");
    let custom_export = EnhancedCsvExport::new(
        data.clone(),
        Some(funding_report),
        backtest.trading_pnl.clone(),
        backtest.funding_pnl.clone(),
        backtest.total_pnl.clone(),
        backtest.position_sizes.clone(),
    );
    
    let custom_file = format!("{}/custom_export.csv", output_dir);
    custom_export.export_to_csv(&custom_file)?;
    println!("Custom export saved to {}", custom_file);
    
    println!("\nExample completed successfully!");
    
    Ok(())
}