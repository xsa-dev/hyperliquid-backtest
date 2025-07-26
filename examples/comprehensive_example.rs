use chrono::{Duration, Utc};
use hyperliquid_backtest::prelude::*;
use rs_backtester::prelude::*;
use std::fs::File;
use std::io::Write;
use tracing::Instrument;

/// # Comprehensive Hyperliquid Backtester Example
///
/// This example demonstrates the full capabilities of the Hyperliquid backtester library,
/// including:
/// - Advanced logging and performance monitoring
/// - Multi-asset data fetching with error handling
/// - Complex strategy development with funding awareness
/// - Comprehensive reporting and analysis
/// - CSV export with enhanced data
/// - Performance comparison across different configurations
///
/// ## Features Demonstrated
///
/// 1. **Structured Logging**: Debug, info, and performance logging
/// 2. **Data Fetching**: Multiple assets with different intervals
/// 3. **Strategy Development**: Funding-aware SMA crossover strategy
/// 4. **Risk Management**: Position sizing and drawdown limits
/// 5. **Performance Analysis**: Detailed metrics and comparisons
/// 6. **Export Capabilities**: CSV export with funding data
///
/// ## Usage
///
/// ```bash
/// # Basic run
/// cargo run --example comprehensive_example
///
/// # With debug logging
/// RUST_LOG=debug cargo run --example comprehensive_example
///
/// # With JSON logging to file
/// RUST_LOG=info HYPERLIQUID_LOG_FORMAT=json HYPERLIQUID_LOG_FILE=backtest.log cargo run --example comprehensive_example
/// ```

#[tokio::main]
async fn main() -> Result<(), HyperliquidBacktestError> {
    // Initialize comprehensive logging
    init_logger_with_level("info");
    
    log::info!("Starting Comprehensive Hyperliquid Backtester Example");
    
    println!("🚀 Comprehensive Hyperliquid Backtester Example");
    println!("===============================================\n");
    
    // Configuration
    let initial_capital = 50000.0; // $50,000 for more realistic testing
    let lookback_days = 60; // 60 days of data
    
    // Calculate time range
    let end_time = Utc::now().timestamp() as u64;
    let start_time = end_time - (lookback_days * 24 * 3600);
    
    println!("📊 Configuration:");
    println!("  Initial Capital: ${:.2}", initial_capital);
    println!("  Lookback Period: {} days", lookback_days);
    println!("  Time Range: {} to {}\n", 
        chrono::DateTime::from_timestamp(start_time as i64, 0).unwrap().format("%Y-%m-%d"),
        chrono::DateTime::from_timestamp(end_time as i64, 0).unwrap().format("%Y-%m-%d"));
    
    // Step 1: Fetch data for multiple assets
    println!("📈 Step 1: Fetching Multi-Asset Data");
    println!("-----------------------------------");
    
    let assets = vec![
        ("BTC", "1h"),
        ("ETH", "1h"),
        ("SOL", "1h"),
    ];
    
    let mut asset_data = Vec::new();
    
    for (symbol, interval) in &assets {
        let span = performance_span("data_fetch", &[
            ("symbol", symbol),
            ("interval", interval),
            ("days", &lookback_days.to_string())
        ]);
        
        let fetch_result = async {
            log::info!("Fetching {} data with {} interval", symbol, interval);
            println!("  Fetching {} data...", symbol);
            
            let data = HyperliquidData::fetch(symbol, interval, start_time, end_time).await?;
            
            log::info!("Successfully fetched {} data points for {}", data.datetime.len(), symbol);
            println!("    ✅ {} data points fetched", data.datetime.len());
            
            Ok::<_, HyperliquidBacktestError>((symbol.to_string(), data))
        }
        .instrument(span)
        .await?;
        
        asset_data.push(fetch_result);
    }
    
    println!("  📊 Data Summary:");
    for (symbol, data) in &asset_data {
        let funding_points = data.funding_rates.len();
        let price_range = data.high.iter().fold(0.0, |a, &b| a.max(b)) - 
                         data.low.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        
        println!("    {}: {} OHLC points, {} funding points, price range: ${:.2}", 
            symbol, data.datetime.len(), funding_points, price_range);
    }
    println!();
    
    // Step 2: Create and test strategies
    println!("🧠 Step 2: Strategy Development and Testing");
    println!("------------------------------------------");
    
    let mut strategy_results = Vec::new();
    
    // Test different strategy configurations
    let strategy_configs = vec![
        ("Conservative SMA (20/50)", 20, 50, 0.05), // 5% funding weight
        ("Aggressive SMA (5/15)", 5, 15, 0.15),     // 15% funding weight
        ("Balanced SMA (10/30)", 10, 30, 0.10),     // 10% funding weight
    ];
    
    for (name, short_period, long_period, funding_weight) in strategy_configs {
        println!("  Testing strategy: {}", name);
        
        let span = performance_span("strategy_backtest", &[
            ("strategy", name),
            ("short_period", &short_period.to_string()),
            ("long_period", &long_period.to_string())
        ]);
        
        let backtest_result = async {
            // Use BTC data for strategy testing
            let (_, btc_data) = asset_data.iter()
                .find(|(symbol, _)| symbol == "BTC")
                .ok_or_else(|| HyperliquidBacktestError::DataConversion("BTC data not found".to_string()))?;
            
            // Create enhanced SMA strategy with funding awareness
            let strategy = enhanced_sma_cross(
                short_period,
                long_period,
                FundingAwareConfig {
                    funding_weight,
                    min_funding_threshold: 0.0001, // 0.01%
                }
            )?;
            
            // Configure commission with realistic Hyperliquid rates
            let commission = HyperliquidCommission {
                maker_rate: 0.0002,  // 0.02%
                taker_rate: 0.0005,  // 0.05%
                funding_enabled: true,
            };
            
            // Create and run backtest
            let mut backtest = HyperliquidBacktest::new(
                btc_data.clone(),
                strategy,
                initial_capital,
                commission,
            )?;
            
            log::info!("Running backtest for strategy: {}", name);
            backtest.calculate_with_funding()?;
            
            let stats = backtest.enhanced_report()?;
            let funding_report = backtest.funding_report()?;
            
            log::info!("Completed backtest for {}: {:.2}% return", name, stats.total_return * 100.0);
            
            Ok::<_, HyperliquidBacktestError>((name.to_string(), stats, funding_report))
        }
        .instrument(span)
        .await?;
        
        strategy_results.push(backtest_result);
        
        let (_, stats, funding_report) = &strategy_results.last().unwrap();
        println!("    📊 Results: {:.2}% return, {:.2}% max drawdown, ${:.2} funding PnL",
            stats.total_return * 100.0,
            stats.max_drawdown * 100.0,
            funding_report.net_funding_pnl);
    }
    println!();
    
    // Step 3: Detailed Analysis
    println!("📊 Step 3: Detailed Performance Analysis");
    println!("---------------------------------------");
    
    // Find best performing strategy
    let best_strategy = strategy_results.iter()
        .max_by(|a, b| a.1.total_return.partial_cmp(&b.1.total_return).unwrap())
        .unwrap();
    
    println!("🏆 Best Performing Strategy: {}", best_strategy.0);
    println!("  Total Return: {:.2}%", best_strategy.1.total_return * 100.0);
    println!("  Sharpe Ratio: {:.3}", best_strategy.1.sharpe_ratio);
    println!("  Max Drawdown: {:.2}%", best_strategy.1.max_drawdown * 100.0);
    println!("  Win Rate: {:.2}%", best_strategy.1.win_rate * 100.0);
    println!();
    
    // Funding analysis
    println!("💰 Funding Rate Analysis:");
    println!("  Net Funding PnL: ${:.2}", best_strategy.2.net_funding_pnl);
    println!("  Total Funding Received: ${:.2}", best_strategy.2.total_funding_received);
    println!("  Total Funding Paid: ${:.2}", best_strategy.2.total_funding_paid);
    println!("  Average Funding Rate: {:.4}%", best_strategy.2.avg_funding_rate * 100.0);
    println!("  Funding Efficiency: {:.2}", best_strategy.2.funding_efficiency);
    println!();
    
    // Step 4: Risk Analysis
    println!("⚠️  Step 4: Risk Analysis");
    println!("------------------------");
    
    for (name, stats, funding_report) in &strategy_results {
        let risk_adjusted_return = stats.total_return / stats.max_drawdown.max(0.01);
        let funding_dependency = funding_report.net_funding_pnl.abs() / stats.total_return.abs().max(0.01);
        
        println!("  {}", name);
        println!("    Risk-Adjusted Return: {:.2}", risk_adjusted_return);
        println!("    Funding Dependency: {:.2}%", funding_dependency * 100.0);
        println!("    Volatility: {:.2}%", stats.volatility * 100.0);
        println!();
    }
    
    // Step 5: Export Results
    println!("💾 Step 5: Exporting Results");
    println!("---------------------------");
    
    // Export detailed results for the best strategy
    let (_, btc_data) = asset_data.iter()
        .find(|(symbol, _)| symbol == "BTC")
        .unwrap();
    
    // Recreate the best strategy for export
    let best_config = strategy_configs.iter()
        .find(|(name, _, _, _)| name == &best_strategy.0)
        .unwrap();
    
    let export_strategy = enhanced_sma_cross(
        best_config.1,
        best_config.2,
        FundingAwareConfig {
            funding_weight: best_config.3,
            min_funding_threshold: 0.0001,
        }
    )?;
    
    let mut export_backtest = HyperliquidBacktest::new(
        btc_data.clone(),
        export_strategy,
        initial_capital,
        HyperliquidCommission::default(),
    )?;
    
    export_backtest.calculate_with_funding()?;
    
    // Export to CSV with enhanced data
    let csv_filename = "comprehensive_backtest_results.csv";
    export_backtest.export_enhanced_csv(csv_filename)?;
    println!("  ✅ Detailed results exported to {}", csv_filename);
    
    // Export strategy comparison
    let comparison_filename = "strategy_comparison.csv";
    let mut comparison_file = File::create(comparison_filename)?;
    
    writeln!(comparison_file, "Strategy,Total Return (%),Sharpe Ratio,Max Drawdown (%),Win Rate (%),Funding PnL ($),Risk-Adjusted Return")?;
    
    for (name, stats, funding_report) in &strategy_results {
        let risk_adjusted_return = stats.total_return / stats.max_drawdown.max(0.01);
        writeln!(comparison_file, "{},{:.2},{:.3},{:.2},{:.2},{:.2},{:.2}",
            name,
            stats.total_return * 100.0,
            stats.sharpe_ratio,
            stats.max_drawdown * 100.0,
            stats.win_rate * 100.0,
            funding_report.net_funding_pnl,
            risk_adjusted_return
        )?;
    }
    
    println!("  ✅ Strategy comparison exported to {}", comparison_filename);
    println!();
    
    // Step 6: Performance Summary
    println!("🎯 Step 6: Final Performance Summary");
    println!("----------------------------------");
    
    println!("📈 Market Data Processed:");
    println!("  Total Assets: {}", asset_data.len());
    println!("  Total Data Points: {}", asset_data.iter().map(|(_, data)| data.datetime.len()).sum::<usize>());
    println!("  Total Funding Points: {}", asset_data.iter().map(|(_, data)| data.funding_rates.len()).sum::<usize>());
    
    println!("\n🧠 Strategies Tested: {}", strategy_results.len());
    
    println!("\n🏆 Best Strategy Performance:");
    println!("  Strategy: {}", best_strategy.0);
    println!("  Initial Capital: ${:.2}", initial_capital);
    println!("  Final Value: ${:.2}", initial_capital * (1.0 + best_strategy.1.total_return));
    println!("  Net Profit: ${:.2}", initial_capital * best_strategy.1.total_return);
    println!("  Total Return: {:.2}%", best_strategy.1.total_return * 100.0);
    
    println!("\n💰 Funding Impact:");
    println!("  Funding PnL: ${:.2}", best_strategy.2.net_funding_pnl);
    println!("  Funding as % of Total Return: {:.2}%", 
        (best_strategy.2.net_funding_pnl / (initial_capital * best_strategy.1.total_return)).abs() * 100.0);
    
    println!("\n📊 Files Generated:");
    println!("  - {}: Detailed backtest results with funding data", csv_filename);
    println!("  - {}: Strategy performance comparison", comparison_filename);
    
    log::info!("Comprehensive example completed successfully");
    println!("\n✅ Comprehensive example completed successfully!");
    println!("   Check the generated CSV files for detailed analysis.");
    
    Ok(())
}