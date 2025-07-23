use chrono::{Duration, Utc};
use hyperliquid_backtester::prelude::*;
use std::fs::File;
use std::io::Write;

/// # Simple OHLC Data Fetching Example
///
/// This example demonstrates how to fetch historical OHLC data from Hyperliquid API
/// for different time intervals and trading pairs. It shows:
/// - Basic data fetching for BTC
/// - Custom time range specification
/// - Data validation and statistics
/// - Saving data to CSV for further analysis
///
/// The example fetches 7 days of hourly data for BTC/USD and saves it to a CSV file.

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hyperliquid Simple Data Fetching Example");
    println!("=======================================\n");

    // Define time range for data fetching (last 7 days)
    let end_time = Utc::now().timestamp() as u64;
    let start_time = end_time - (7 * 24 * 3600); // 7 days of data
    
    println!("Fetching BTC/USD hourly data for the last 7 days...");
    
    // Fetch BTC data using the convenience method
    let btc_data = HyperliquidData::fetch_btc("1h", start_time, end_time).await?;
    
    // Print data statistics
    println!("\nData fetched successfully!");
    println!("Symbol: {}", btc_data.ticker);
    println!("Time range: {} to {}", 
        btc_data.datetime.first().unwrap().format("%Y-%m-%d %H:%M"),
        btc_data.datetime.last().unwrap().format("%Y-%m-%d %H:%M"));
    println!("Number of data points: {}", btc_data.len());
    println!("Number of funding rate points: {}", btc_data.funding_rates.len());
    
    // Calculate some basic statistics
    let mut min_price = f64::MAX;
    let mut max_price = f64::MIN;
    let mut total_volume = 0.0;
    
    for i in 0..btc_data.len() {
        min_price = min_price.min(btc_data.low[i]);
        max_price = max_price.max(btc_data.high[i]);
        total_volume += btc_data.volume[i];
    }
    
    println!("\nPrice range: ${:.2} - ${:.2}", min_price, max_price);
    println!("Total volume: {:.2}", total_volume);
    
    // Calculate average funding rate
    let mut total_funding = 0.0;
    let mut funding_count = 0;
    
    for rate in &btc_data.funding_rates {
        if !rate.is_nan() {
            total_funding += rate;
            funding_count += 1;
        }
    }
    
    let avg_funding = if funding_count > 0 {
        total_funding / funding_count as f64
    } else {
        0.0
    };
    
    println!("Average funding rate: {:.6}%", avg_funding * 100.0);
    
    // Save data to CSV
    println!("\nSaving data to CSV file...");
    let mut csv_content = String::from("timestamp,datetime,open,high,low,close,volume,funding_rate\n");
    
    for i in 0..btc_data.len() {
        let timestamp = btc_data.datetime[i].timestamp();
        let datetime = btc_data.datetime[i].format("%Y-%m-%d %H:%M:%S").to_string();
        let open = btc_data.open[i];
        let high = btc_data.high[i];
        let low = btc_data.low[i];
        let close = btc_data.close[i];
        let volume = btc_data.volume[i];
        
        // Find the closest funding rate for this timestamp
        let funding_rate = btc_data.get_funding_rate_at(btc_data.datetime[i])
            .unwrap_or(f64::NAN);
        
        csv_content.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            timestamp, datetime, open, high, low, close, volume, funding_rate
        ));
    }
    
    let csv_file = "btc_data.csv";
    let mut file = File::create(csv_file)?;
    file.write_all(csv_content.as_bytes())?;
    println!("Data saved to {}", csv_file);
    
    // Demonstrate fetching data for another asset (ETH)
    println!("\nFetching ETH/USD data for comparison...");
    let eth_data = HyperliquidData::fetch("ETH", "1h", start_time, end_time).await?;
    
    println!("ETH data fetched: {} data points", eth_data.len());
    
    // Demonstrate fetching data with different time interval
    println!("\nFetching BTC/USD data with 5-minute intervals...");
    let btc_5m_data = HyperliquidData::fetch_btc("5m", end_time - (24 * 3600), end_time).await?;
    
    println!("BTC 5-minute data fetched: {} data points for the last 24 hours", btc_5m_data.len());
    
    println!("\nExample completed successfully!");
    
    Ok(())
}