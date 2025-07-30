use chrono::Utc;
use hyperliquid_backtest::prelude::*;

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
async fn main() -> Result<()> {
    // Initialize logging for better debugging and monitoring
    init_logger_with_level("info");
    
    log::info!("Starting Hyperliquid Basic Backtest Example");
    
    println!("🚀 Hyperliquid Basic Backtest Example");
    println!("=====================================\n");

    // Fetch historical data for BTC with funding rates
    let end_time = Utc::now().timestamp() as u64;
    let start_time = end_time - (90 * 24 * 3600); // 90 days of data
    
    println!("Fetching BTC/USD data for the last 90 days...");
    let data = HyperliquidData::fetch("BTC", "1h", start_time, end_time).await?;
    
    println!("Data fetched: {} data points from {} to {}\n", 
        data.len(),
        data.datetime.first().unwrap().format("%Y-%m-%d %H:%M"),
        data.datetime.last().unwrap().format("%Y-%m-%d %H:%M"));
    
    // Create a simple SMA crossover strategy using the enhanced_sma_cross function
    println!("Setting up SMA crossover strategy (10/30)...");
    let strategy = enhanced_sma_cross(data.to_rs_backtester_data(), 10, 30, Default::default());
    
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
        "SMA Crossover (10/30)".to_string(),
        initial_capital,
        commission.clone(),
    );
    
    // Run backtest with funding rates
    backtest.calculate_with_funding()?;
    
    // Get enhanced report
    let report = backtest.enhanced_report()?;
    
    // Print backtest results
    println!("\nBacktest Results:");
    println!("----------------");
    println!("Initial Capital: ${:.2}", initial_capital);
    println!("Final Equity: ${:.2}", report.final_equity);
    println!("Total Return: {:.2}%", report.total_return * 100.0);
    println!("Max Drawdown: {:.2}%", report.max_drawdown * 100.0);
    println!("Win Rate: {:.2}%", report.win_rate * 100.0);
    println!("Profit Factor: {:.2}", report.profit_factor);
    println!("Sharpe Ratio: {:.2}", report.sharpe_ratio);
    
    // Get funding impact from enhanced metrics
    let enhanced_metrics = &report.enhanced_metrics;
    println!("\nFunding Rate Impact:");
    println!("------------------");
    println!("Total Return with Funding: {:.2}%", enhanced_metrics.total_return_with_funding * 100.0);
    println!("Trading Only Return: {:.2}%", enhanced_metrics.trading_only_return * 100.0);
    println!("Funding Only Return: {:.2}%", enhanced_metrics.funding_only_return * 100.0);
    println!("Funding Payments Received: {}", enhanced_metrics.funding_payments_received);
    println!("Funding Payments Paid: {}", enhanced_metrics.funding_payments_paid);
    println!("Average Funding Rate: {:.4}%", enhanced_metrics.average_funding_rate * 100.0);
    
    // Get commission statistics
    let commission_stats = &report.commission_stats;
    println!("\nCommission Statistics:");
    println!("-------------------");
    println!("Total Commission: ${:.2}", commission_stats.total_commission);
    println!("Maker Fees: ${:.2}", commission_stats.maker_fees);
    println!("Taker Fees: ${:.2}", commission_stats.taker_fees);
    println!("Maker/Taker Ratio: {:.2}", commission_stats.maker_taker_ratio);
    
    // Get funding summary
    let funding_summary = &report.funding_summary;
    println!("\nFunding Summary:");
    println!("---------------");
    println!("Total Funding Paid: ${:.2}", funding_summary.total_funding_paid);
    println!("Total Funding Received: ${:.2}", funding_summary.total_funding_received);
    println!("Net Funding: ${:.2}", funding_summary.net_funding);
    println!("Funding Contribution: {:.2}%", funding_summary.funding_contribution_percentage * 100.0);
    
    // Export results to CSV
    println!("\nExporting results to CSV...");
    backtest.export_to_csv("basic_backtest_results.csv")?;
    println!("Results exported to basic_backtest_results.csv");
    
    // Run the same backtest without funding to compare
    println!("\nRunning comparison backtest without funding rates...");
    let mut commission_no_funding = commission.clone();
    commission_no_funding.funding_enabled = false;
    
    let mut backtest_no_funding = HyperliquidBacktest::new(
        data.clone(),
        "SMA Crossover (10/30) - No Funding".to_string(),
        initial_capital,
        commission_no_funding,
    );
    
    backtest_no_funding.calculate_with_funding()?;
    
    let report_no_funding = backtest_no_funding.enhanced_report()?;
    
    println!("\nComparison Results (Without Funding):");
    println!("-----------------------------------");
    println!("Total Return: {:.2}%", report_no_funding.total_return * 100.0);
    println!("Final Equity: ${:.2}", report_no_funding.final_equity);
    
    println!("\nFunding Impact on Performance:");
    println!("----------------------------");
    println!("Return Difference: {:.2}%", 
        (report.total_return - report_no_funding.total_return) * 100.0);
    println!("Equity Difference: ${:.2}", 
        report.final_equity - report_no_funding.final_equity);
    println!("Performance Impact: {:.2}%", 
        ((report.total_return / report_no_funding.total_return) - 1.0) * 100.0);
    
    // Print detailed funding report
    println!("\nDetailed Funding Analysis:");
    println!("-------------------------");
    let funding_report = backtest.funding_report()?;
    println!("Total Funding Received: ${:.2}", funding_report.total_funding_received);
    println!("Total Funding Paid: ${:.2}", funding_report.total_funding_paid);
    println!("Net Funding PnL: ${:.2}", funding_report.net_funding_pnl);
    println!("Payment Count: {}", funding_report.payment_count);
    println!("Average Rate: {:.4}%", funding_report.average_rate * 100.0);
    
    println!("\nExample completed successfully!");
    
    Ok(())
}