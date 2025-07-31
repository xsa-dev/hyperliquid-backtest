use chrono::{Duration, TimeZone, Utc, FixedOffset};
use hyperliquid_rust_sdk::{BaseUrl, InfoClient};
use hyperliquid_backtest::prelude::*;
use std::fs::File;
use std::io::Write;

/// # Simple OHLC Data Fetching Example (Fixed with Working API)
///
/// This example demonstrates how to fetch historical OHLC data from Hyperliquid API
/// for different time intervals and trading pairs. It shows:
/// - Basic data fetching for BTC using the correct SDK
/// - Custom time range specification
/// - Data validation and statistics
/// - Saving data to CSV for further analysis
/// - Testing different symbols and intervals
///
/// The example fetches 7 days of hourly data for BTC/USD and saves it to a CSV file.

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Hyperliquid Simple Data Fetching Example (Fixed)");
    println!("==================================================\n");

    // Define time range for data fetching (last 7 days)
    let end_time = Utc::now();
    let start_time = end_time - Duration::days(7);
    let start_timestamp = start_time.timestamp_millis() as u64;
    let end_timestamp = end_time.timestamp_millis() as u64;
    
    println!("Fetching BTC/USD hourly data for the last 7 days...");
    println!("Time range: {} to {}", 
        start_time.format("%Y-%m-%d %H:%M"),
        end_time.format("%Y-%m-%d %H:%M"));

    // Initialize Hyperliquid client
    let info_client = InfoClient::new(None, Some(BaseUrl::Mainnet)).await?;
    
    // Fetch BTC data using the working SDK
    let candles = info_client
        .candles_snapshot("BTC".to_string(), "1h".to_string(), start_timestamp, end_timestamp)
        .await?;

    println!("✅ Successfully fetched {} candles!", candles.len());

    if candles.is_empty() {
        return Err(HyperliquidBacktestError::api_error("No data received from API"));
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
    let btc_data = HyperliquidData::with_ohlc_data(
        "BTC".to_string(),
        datetime,
        open,
        high,
        low,
        close,
        volume,
    )?;
    
    // Print data statistics
    println!("\nData fetched successfully!");
    println!("Symbol: {}", btc_data.symbol);
    println!("Time range: {} to {}", 
        btc_data.datetime.first().map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "N/A".to_string()),
        btc_data.datetime.last().map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "N/A".to_string()));
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
    
    // Try fetching different time intervals
    println!("\nFetching 5-minute data for the last 24 hours...");
    let btc_5m_candles = info_client
        .candles_snapshot("BTC".to_string(), "5m".to_string(), end_timestamp - (24 * 3600 * 1000), end_timestamp)
        .await?;
    println!("5m data points: {}", btc_5m_candles.len());
    
    // Try fetching ETH data
    println!("\nFetching ETH data for comparison...");
    let eth_candles = info_client
        .candles_snapshot("ETH".to_string(), "1h".to_string(), start_timestamp, end_timestamp)
        .await?;
    println!("ETH data points: {}", eth_candles.len());
    
    // Try fetching SOL data
    println!("\nFetching SOL data for comparison...");
    let sol_candles = info_client
        .candles_snapshot("SOL".to_string(), "1h".to_string(), start_timestamp, end_timestamp)
        .await?;
    println!("SOL data points: {}", sol_candles.len());
    
    // Test different intervals for BTC
    println!("\nTesting different intervals for BTC...");
    let intervals = vec!["1m", "5m", "15m", "1h", "4h", "1d"];
    
    for interval in intervals {
        println!("Testing interval: {}", interval);
        match info_client
            .candles_snapshot("BTC".to_string(), interval.to_string(), start_timestamp, end_timestamp)
            .await {
            Ok(test_candles) => {
                println!("  ✅ {}: {} candles", interval, test_candles.len());
            }
            Err(e) => {
                println!("  ❌ {}: {}", interval, e);
            }
        }
    }
    
    println!("\nExample completed successfully!");
    
    Ok(())
}