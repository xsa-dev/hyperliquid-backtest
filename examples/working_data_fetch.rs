use chrono::{Duration, TimeZone, Utc};
use hyperliquid_rust_sdk::{BaseUrl, InfoClient};
use std::fs::File;
use std::io::Write;

/// Working example for fetching Hyperliquid data using the correct SDK approach
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Working Hyperliquid Data Fetch Example");
    println!("========================================\n");

    // Settings
    let coin = "BTC";
    let interval = "1h"; // 1 hour candles
    let now = Utc::now();
    let start_time = (now - Duration::days(7)).timestamp_millis() as u64; // 7 days ago
    let end_time = now.timestamp_millis() as u64;

    // Initialize client
    println!("Initializing Hyperliquid client...");
    let info_client = InfoClient::new(None, Some(BaseUrl::Mainnet)).await?;
    println!("✅ Client initialized successfully!");

    // Fetch OHLCV data
    println!("\nFetching {}-USDC candles for the last 7 days...", coin);
    let candles = info_client
        .candles_snapshot(coin.to_string(), interval.to_string(), start_time, end_time)
        .await?;
    
    println!("✅ Successfully fetched {} candles!", candles.len());
    
    if !candles.is_empty() {
        let first_candle = &candles[0];
        let last_candle = &candles[candles.len() - 1];
        
        println!("Time range: {} to {}", 
            Utc.timestamp_millis_opt(first_candle.time_open as i64).unwrap().format("%Y-%m-%d %H:%M"),
            Utc.timestamp_millis_opt(last_candle.time_close as i64).unwrap().format("%Y-%m-%d %H:%M"));
        
        // Parse string values to f64 for calculations
        let prices: Vec<f64> = candles.iter()
            .filter_map(|c| c.low.parse::<f64>().ok())
            .collect();
        
        if !prices.is_empty() {
            let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            println!("Price range: ${:.2} - ${:.2}", min_price, max_price);
        }
    }

    // Save to CSV
    println!("\nSaving data to CSV...");
    let mut file = File::create("btc_usdc_ohlcv_1h.csv")?;
    writeln!(file, "time_open,time_close,open,high,low,close,volume,num_trades")?;
    
    for c in &candles {
        writeln!(
            file,
            "{},{},{},{},{},{},{},{}",
            c.time_open, c.time_close, c.open, c.high, c.low, c.close, c.vlm, c.num_trades
        )?;
    }
    println!("✅ Data saved to btc_usdc_ohlcv_1h.csv");

    // Test different coins
    println!("\nTesting different coins...");
    let coins = vec!["ETH", "SOL", "AVAX", "MATIC", "ATOM"];
    
    for test_coin in coins {
        println!("Testing {}...", test_coin);
        match info_client
            .candles_snapshot(test_coin.to_string(), "1h".to_string(), start_time, end_time)
            .await {
            Ok(test_candles) => {
                println!("  ✅ {}: {} candles", test_coin, test_candles.len());
            }
            Err(e) => {
                println!("  ❌ {}: {}", test_coin, e);
            }
        }
    }

    // Test different intervals
    println!("\nTesting different intervals for BTC...");
    let intervals = vec!["1m", "5m", "15m", "1h", "4h", "1d"];
    
    for test_interval in intervals {
        println!("Testing {} interval...", test_interval);
        match info_client
            .candles_snapshot("BTC".to_string(), test_interval.to_string(), start_time, end_time)
            .await {
            Ok(test_candles) => {
                println!("  ✅ {}: {} candles", test_interval, test_candles.len());
            }
            Err(e) => {
                println!("  ❌ {}: {}", test_interval, e);
            }
        }
    }

    // Fetch orderbook snapshot
    println!("\nFetching orderbook snapshot for BTC...");
    match info_client.l2_snapshot("BTC".to_string()).await {
        Ok(l2) => {
            println!("✅ Orderbook snapshot received!");
            if l2.levels.len() == 2 {
                let bid_levels = &l2.levels[0];
                let ask_levels = &l2.levels[1];
                println!("  Bids: {} levels", bid_levels.len());
                println!("  Asks: {} levels", ask_levels.len());
                
                if !bid_levels.is_empty() && !ask_levels.is_empty() {
                    println!("  Best bid: ${} (size: {})", bid_levels[0].px, bid_levels[0].sz);
                    println!("  Best ask: ${} (size: {})", ask_levels[0].px, ask_levels[0].sz);
                    
                    // Parse prices to calculate spread
                    if let (Ok(bid_price), Ok(ask_price)) = (bid_levels[0].px.parse::<f64>(), ask_levels[0].px.parse::<f64>()) {
                        println!("  Spread: ${:.2}", ask_price - bid_price);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to get orderbook: {}", e);
        }
    }

    println!("\n🎉 Example completed successfully!");
    
    Ok(())
} 