use chrono::{DateTime, Duration, FixedOffset, Utc};
use hyperliquid_backtester::prelude::*;
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
    
    // Export detailed funding analysis (new)
    let detailed_analysis_file = format!("{}/detailed_funding_analysis.csv", output_dir);
    backtest.export_detailed_funding_analysis(&detailed_analysis_file)?;
    println!("Detailed funding analysis exported to {}", detailed_analysis_file);
    
    // Export funding impact analysis (new)
    let impact_analysis_file = format!("{}/funding_impact_analysis.csv", output_dir);
    backtest.export_funding_impact_analysis(&impact_analysis_file)?;
    println!("Funding impact analysis exported to {}", impact_analysis_file);
    
    // Export funding correlation matrix (new)
    let correlation_file = format!("{}/funding_correlation.csv", output_dir);
    backtest.export_funding_correlation_matrix(&correlation_file, &["ETH", "SOL"])?;
    println!("Funding correlation matrix exported to {}", correlation_file);
    
    // Export all funding data at once
    println!("\nExporting all funding data at once...");
    let base_path = format!("{}/all_data", output_dir);
    let exported_files = backtest.export_all_funding_data(&base_path)?;
    
    println!("Exported {} files:", exported_files.len());
    for file in exported_files {
        println!("- {}", file);
    }
    
    // Demonstrate strategy comparison export (new)
    println!("\nDemonstrating strategy comparison export...");
    
    // Create a second strategy for comparison
    println!("Running backtest with funding arbitrage strategy...");
    let mut funding_arb_strategy = Strategy::new();
    
    funding_arb_strategy.init(Box::new(move |_ctx, _data| {
        // Initialize strategy
    }));
    
    funding_arb_strategy.next(Box::new(move |ctx, data| {
        if data.index < 10 {
            return;
        }
        
        // Get funding rate (simplified example)
        let funding_rate = data.get_funding_rate_at(data.datetime[data.index]).unwrap_or(0.0);
        
        // Get current position
        let position = ctx.position();
        
        // Simple funding arbitrage strategy
        if funding_rate > 0.0005 && position >= 0.0 { // High positive funding rate
            // Go short to collect funding
            ctx.entry_qty(-1.0);
        } else if funding_rate < -0.0005 && position <= 0.0 { // High negative funding rate
            // Go long to collect funding
            ctx.entry_qty(1.0);
        } else if funding_rate.abs() < 0.0001 && position != 0.0 {
            // Close position when funding rate is near zero
            ctx.close();
        }
    }));
    
    // Run second backtest
    let mut funding_arb_backtest = HyperliquidBacktest::new(
        data.clone(),
        funding_arb_strategy,
        10000.0, // $10,000 initial capital
        commission,
    );
    
    funding_arb_backtest.calculate_with_funding();
    
    // Create strategy comparison data
    let strategy_comparison = StrategyComparisonData {
        strategy_names: vec!["SMA_Cross".to_string(), "Funding_Arbitrage".to_string()],
        initial_capitals: vec![10000.0, 10000.0],
        final_equities: vec![
            10000.0 + backtest.trading_pnl.iter().sum::<f64>() + backtest.funding_pnl.iter().sum::<f64>(),
            10000.0 + funding_arb_backtest.trading_pnl.iter().sum::<f64>() + funding_arb_backtest.funding_pnl.iter().sum::<f64>(),
        ],
        total_returns: vec![
            (backtest.trading_pnl.iter().sum::<f64>() + backtest.funding_pnl.iter().sum::<f64>()) / 10000.0 * 100.0,
            (funding_arb_backtest.trading_pnl.iter().sum::<f64>() + funding_arb_backtest.funding_pnl.iter().sum::<f64>()) / 10000.0 * 100.0,
        ],
        trading_pnls: vec![
            backtest.trading_pnl.iter().sum::<f64>(),
            funding_arb_backtest.trading_pnl.iter().sum::<f64>(),
        ],
        funding_pnls: vec![
            backtest.funding_pnl.iter().sum::<f64>(),
            funding_arb_backtest.funding_pnl.iter().sum::<f64>(),
        ],
        funding_impacts: vec![
            backtest.funding_pnl.iter().sum::<f64>() / (backtest.trading_pnl.iter().sum::<f64>() + backtest.funding_pnl.iter().sum::<f64>() + 1e-10) * 100.0,
            funding_arb_backtest.funding_pnl.iter().sum::<f64>() / (funding_arb_backtest.trading_pnl.iter().sum::<f64>() + funding_arb_backtest.funding_pnl.iter().sum::<f64>() + 1e-10) * 100.0,
        ],
        max_drawdowns: vec![5.0, 3.0], // Placeholder values
        sharpe_ratios: vec![1.5, 1.2],  // Placeholder values
        sortino_ratios: vec![2.0, 1.8], // Placeholder values
        calmar_ratios: vec![4.0, 5.0],  // Placeholder values
        funding_adjusted_sharpes: vec![1.4, 1.3], // Placeholder values
    };
    
    // Export strategy comparison
    let comparison_file = format!("{}/strategy_comparison.csv", output_dir);
    EnhancedCsvExport::export_strategy_comparison(&[strategy_comparison], &comparison_file)?;
    println!("Strategy comparison exported to {}", comparison_file);
    
    // Demonstrate usage of the new EnhancedCsvExport::new_with_strategy constructor
    println!("\nCreating custom CSV export with strategy information...");
    let custom_export = EnhancedCsvExport::new_with_strategy(
        data.clone(),
        Some(funding_report),
        backtest.trading_pnl.clone(),
        backtest.funding_pnl.clone(),
        backtest.trading_pnl.iter().zip(backtest.funding_pnl.iter()).map(|(t, f)| t + f).collect(),
        backtest.position_sizes.clone(),
        "SMA_Cross_Strategy".to_string(),
        10000.0,
    );
    
    let custom_file = format!("{}/custom_strategy_export.csv", output_dir);
    custom_export.export_to_csv(&custom_file)?;
    println!("Custom strategy export saved to {}", custom_file);
    
    println!("\nEnhanced CSV Export Example completed successfully!");
    
    Ok(())
}