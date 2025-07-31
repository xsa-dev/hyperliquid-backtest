use chrono::{Duration, TimeZone, Utc, FixedOffset};
use hyperliquid_rust_sdk::{BaseUrl, InfoClient};
use hyperliquid_backtest::prelude::*;

/// Simple Working Backtest Example
///
/// This example demonstrates how to:
/// 1. Fetch data from Hyperliquid API using the working SDK
/// 2. Convert data to our internal format
/// 3. Run a simple backtest
/// 4. Display results
///
/// This uses only our own code without external dependencies.

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Simple Working Backtest Example");
    println!("==================================\n");

    // Define time range (last 7 days for faster testing)
    let end_time = Utc::now();
    let start_time = end_time - Duration::days(7);
    let start_timestamp = start_time.timestamp_millis() as u64;
    let end_timestamp = end_time.timestamp_millis() as u64;

    println!("Fetching BTC/USD data for the last 7 days...");
    println!("Time range: {} to {}", 
        start_time.format("%Y-%m-%d %H:%M"),
        end_time.format("%Y-%m-%d %H:%M"));

    // Initialize Hyperliquid client
    let info_client = InfoClient::new(None, Some(BaseUrl::Mainnet)).await?;
    
    // Fetch OHLCV data
    let candles = info_client
        .candles_snapshot("BTC".to_string(), "1h".to_string(), start_timestamp, end_timestamp)
        .await?;

    println!("✅ Successfully fetched {} candles!", candles.len());

    if candles.is_empty() {
        println!("❌ No data received from API");
        return Ok(());
    }

    // Convert candles to our internal format
    let mut datetime = Vec::new();
    let mut open = Vec::new();
    let mut high = Vec::new();
    let mut low = Vec::new();
    let mut close = Vec::new();
    let mut volume = Vec::new();

    for candle in &candles {
        let timestamp = Utc.timestamp_millis_opt(candle.time_open as i64).unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap());
        
        datetime.push(timestamp);
        open.push(candle.open.parse::<f64>().unwrap_or(0.0));
        high.push(candle.high.parse::<f64>().unwrap_or(0.0));
        low.push(candle.low.parse::<f64>().unwrap_or(0.0));
        close.push(candle.close.parse::<f64>().unwrap_or(0.0));
        volume.push(candle.vlm.parse::<f64>().unwrap_or(0.0));
    }

    // Create our internal Data struct
    let data = HyperliquidData::with_ohlc_data(
        "BTC".to_string(),
        datetime,
        open,
        high,
        low,
        close,
        volume,
    )?;

    println!("Data converted: {} data points from {} to {}\n", 
        data.len(),
        data.datetime.first().map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "N/A".to_string()),
        data.datetime.last().map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "N/A".to_string()));

    // Create a simple SMA crossover strategy
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

    // Initialize the base backtest first
    backtest.initialize_base_backtest()?;
    
    // Then calculate with funding rates
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
    backtest.export_to_csv("simple_backtest_results.csv")?;
    println!("Results exported to simple_backtest_results.csv");

    // Print detailed funding report
    println!("\nDetailed Funding Analysis:");
    println!("-------------------------");
    let funding_report = backtest.funding_report()?;
    println!("Total Funding Received: ${:.2}", funding_report.total_funding_received);
    println!("Total Funding Paid: ${:.2}", funding_report.total_funding_paid);
    println!("Net Funding PnL: ${:.2}", funding_report.net_funding_pnl);
    println!("Payment Count: {}", funding_report.payment_count);
    println!("Average Rate: {:.4}%", funding_report.average_rate * 100.0);

    println!("\n🎉 Example completed successfully!");
    
    Ok(())
} 