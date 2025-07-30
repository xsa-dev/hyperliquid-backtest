use chrono::{Duration, Utc};
use hyperliquid_backtest::prelude::*;
use hyperliquid_backtest::indicators::*;
use std::fs::File;
use std::io::Write;

/// # Funding Indicators Example
///
/// This example demonstrates how to use various funding rate indicators
/// to analyze market conditions and potentially inform trading strategies.
/// It shows:
/// - Calculating funding volatility
/// - Detecting funding momentum
/// - Identifying funding arbitrage opportunities
/// - Correlating funding rates with price movements
///
/// The example analyzes BTC/USD funding data and exports the results to CSV.

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hyperliquid Funding Indicators Example");
    println!("====================================\n");

    // Fetch historical data for BTC with funding rates
    let end_time = Utc::now().timestamp() as u64;
    let start_time = end_time - (60 * 24 * 3600); // 60 days of data
    
    println!("Fetching BTC/USD data for the last 60 days...");
    let data = HyperliquidData::fetch_btc("1h", start_time, end_time).await?;
    
    println!("Data fetched: {} data points from {} to {}\n", 
        data.len(),
        data.datetime.first().unwrap().format("%Y-%m-%d %H:%M"),
        data.datetime.last().unwrap().format("%Y-%m-%d %H:%M"));
    
    // Extract funding rates
    let mut funding_rates = Vec::new();
    let mut timestamps = Vec::new();
    
    for i in 0..data.len() {
        if !data.funding_rates[i].is_nan() {
            funding_rates.push(data.funding_rates[i]);
            timestamps.push(data.datetime[i]);
        }
    }
    
    println!("Found {} valid funding rate data points", funding_rates.len());
    
    // Calculate funding volatility
    println!("\nCalculating funding rate volatility...");
    let window_size = 24; // 24 hours
    let volatility = calculate_funding_volatility(&funding_rates, window_size);
    
    println!("Average funding volatility: {:.6}%", volatility.avg_volatility * 100.0);
    println!("Max funding volatility: {:.6}% on {}", 
        volatility.max_volatility * 100.0,
        timestamps[volatility.max_volatility_index].format("%Y-%m-%d %H:%M"));
    
    // Calculate funding momentum
    println!("\nCalculating funding rate momentum...");
    let momentum_window = 8; // 8 hours
    let momentum = calculate_funding_momentum(&funding_rates, momentum_window);
    
    println!("Current funding momentum: {:.6}", momentum.current_momentum);
    println!("Momentum direction: {}", 
        if momentum.is_increasing {
            "Increasing (Bullish)"
        } else {
            "Decreasing (Bearish)"
        });
    
    // Detect funding arbitrage opportunities
    println!("\nDetecting funding arbitrage opportunities...");
    let threshold = 0.0005; // 0.05% per 8h
    let arb_opportunities = calculate_funding_arbitrage(&funding_rates, threshold);
    
    println!("Found {} potential arbitrage opportunities", arb_opportunities.opportunities.len());
    
    if !arb_opportunities.opportunities.is_empty() {
        println!("Top 3 opportunities:");
        for i in 0..std::cmp::min(3, arb_opportunities.opportunities.len()) {
            let opp = &arb_opportunities.opportunities[i];
            println!("  - Rate: {:.6}%, Expected return: {:.2}% (annualized)",
                opp.funding_rate * 100.0,
                opp.expected_annual_return * 100.0);
        }
    }
    
    // Calculate basis indicator
    println!("\nCalculating basis indicator...");
    let basis = calculate_basis_indicator(&data);
    
    println!("Average basis: {:.6}%", basis.avg_basis * 100.0);
    println!("Current basis trend: {}", 
        match basis.trend {
            BasisTrend::Widening => "Widening (Bullish)",
            BasisTrend::Narrowing => "Narrowing (Bearish)",
            BasisTrend::Stable => "Stable (Neutral)",
        });
    
    // Create funding prediction model
    println!("\nInitializing funding prediction model...");
    let config = FundingPredictionConfig {
        lookback_periods: 24,
        volatility_weight: 0.3,
        momentum_weight: 0.4,
        basis_weight: 0.3,
    };
    
    let mut prediction_model = FundingPredictionModel::new(config);
    
    // Train model with historical data
    println!("Training prediction model with historical data...");
    for i in 0..funding_rates.len() {
        if i >= config.lookback_periods {
            prediction_model.add_observation(funding_rates[i]);
        }
    }
    
    // Make prediction
    let prediction = prediction_model.predict();
    
    println!("\nFunding Rate Prediction:");
    println!("Direction: {}", match prediction.direction {
        FundingDirection::Positive => "Positive (Short pays Long)",
        FundingDirection::Negative => "Negative (Long pays Short)",
        FundingDirection::Neutral => "Neutral",
    });
    println!("Confidence: {:.2}%", prediction.confidence * 100.0);
    println!("Predicted rate: {:.6}%", prediction.predicted_rate * 100.0);
    
    // Export indicators to CSV
    println!("\nExporting funding indicators to CSV...");
    
    let mut csv_content = String::from("timestamp,datetime,price,funding_rate,volatility,momentum,arbitrage_signal,basis\n");
    
    let mut vol_index = 0;
    let mut mom_index = 0;
    let mut arb_index = 0;
    
    for i in 0..data.len() {
        if data.funding_rates[i].is_nan() {
            continue;
        }
        
        let timestamp = data.datetime[i].timestamp();
        let datetime = data.datetime[i].format("%Y-%m-%d %H:%M:%S").to_string();
        let price = data.close[i];
        let funding_rate = data.funding_rates[i];
        
        // Get volatility if available
        let vol_value = if vol_index < volatility.values.len() {
            volatility.values[vol_index]
        } else {
            f64::NAN
        };
        
        // Get momentum if available
        let mom_value = if mom_index < momentum.values.len() {
            momentum.values[mom_index]
        } else {
            f64::NAN
        };
        
        // Check if this is an arbitrage opportunity
        let arb_signal = if arb_index < arb_opportunities.opportunities.len() && 
                          arb_opportunities.opportunities[arb_index].index == i {
            arb_index += 1;
            1.0
        } else {
            0.0
        };
        
        // Get basis value
        let basis_value = basis.values.get(i).copied().unwrap_or(f64::NAN);
        
        csv_content.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            timestamp, datetime, price, funding_rate, vol_value, mom_value, arb_signal, basis_value
        ));
        
        vol_index += 1;
        mom_index += 1;
    }
    
    let csv_file = "funding_indicators.csv";
    let mut file = File::create(csv_file)?;
    file.write_all(csv_content.as_bytes())?;
    println!("Indicators exported to {}", csv_file);
    
    println!("\nExample completed successfully!");
    
    Ok(())
}