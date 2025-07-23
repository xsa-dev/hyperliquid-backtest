use hyperliquid_backtester::prelude::*;
use chrono::Utc;

/// # Getting Started with Hyperliquid Backtester
///
/// This is the simplest possible example to get you started with the Hyperliquid backtester.
/// It demonstrates:
/// - Basic data fetching
/// - Simple strategy creation
/// - Running a backtest
/// - Viewing results
///
/// Perfect for beginners who want to understand the core concepts.
///
/// ## Usage
///
/// ```bash
/// cargo run --example getting_started
/// ```

#[tokio::main]
async fn main() -> Result<(), HyperliquidBacktestError> {
    // Optional: Enable logging to see what's happening
    init_logger();
    
    println!("🚀 Getting Started with Hyperliquid Backtester\n");
    
    // Step 1: Fetch some data
    println!("📊 Fetching BTC data for the last 7 days...");
    
    let end_time = Utc::now().timestamp() as u64;
    let start_time = end_time - (7 * 24 * 60 * 60); // 7 days ago
    
    let data = HyperliquidData::fetch("BTC", "1h", start_time, end_time).await?;
    
    println!("   ✅ Got {} data points", data.datetime.len());
    println!("   📈 Price range: ${:.2} - ${:.2}", 
        data.low.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
        data.high.iter().fold(0.0, |a, &b| a.max(b)));
    
    // Step 2: Create a simple strategy
    println!("\n🧠 Creating a simple buy-and-hold strategy...");
    
    let strategy = enhanced_sma_cross(
        5,   // Short period (5 hours)
        20,  // Long period (20 hours)
        Default::default() // Use default funding awareness
    )?;
    
    // Step 3: Run the backtest
    println!("\n⚡ Running backtest...");
    
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0, // Start with $10,000
        HyperliquidCommission::default(), // Use realistic Hyperliquid fees
    )?;
    
    // Calculate results including funding payments
    backtest.calculate_with_funding()?;
    
    // Step 4: View the results
    println!("\n📊 Results:");
    println!("===========");
    
    let report = backtest.enhanced_report()?;
    
    println!("💰 Performance:");
    println!("   Initial Capital: $10,000.00");
    println!("   Final Value: ${:.2}", 10000.0 * (1.0 + report.total_return));
    println!("   Total Return: {:.2}%", report.total_return * 100.0);
    println!("   Max Drawdown: {:.2}%", report.max_drawdown * 100.0);
    
    let funding_report = backtest.funding_report()?;
    
    println!("\n💸 Funding Impact:");
    println!("   Net Funding PnL: ${:.2}", funding_report.net_funding_pnl);
    println!("   Avg Funding Rate: {:.4}%", funding_report.avg_funding_rate * 100.0);
    
    // Step 5: Simple interpretation
    println!("\n🎯 What does this mean?");
    
    if report.total_return > 0.0 {
        println!("   ✅ Your strategy made money! 🎉");
    } else {
        println!("   ❌ Your strategy lost money. 😞");
    }
    
    if funding_report.net_funding_pnl > 0.0 {
        println!("   💰 You earned money from funding rates!");
    } else if funding_report.net_funding_pnl < 0.0 {
        println!("   💸 You paid money in funding rates.");
    } else {
        println!("   ⚖️  Funding rates had no net impact.");
    }
    
    if report.max_drawdown < 0.05 {
        println!("   🛡️  Low risk: Max drawdown under 5%");
    } else if report.max_drawdown < 0.15 {
        println!("   ⚠️  Medium risk: Max drawdown {:.1}%", report.max_drawdown * 100.0);
    } else {
        println!("   🚨 High risk: Max drawdown {:.1}%", report.max_drawdown * 100.0);
    }
    
    println!("\n🎓 Next Steps:");
    println!("   - Try different time periods or assets");
    println!("   - Experiment with strategy parameters");
    println!("   - Run: cargo run --example comprehensive_example");
    println!("   - Check out other examples in the examples/ directory");
    
    println!("\n✅ Getting started example completed!");
    
    Ok(())
}