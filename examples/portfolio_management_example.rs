use chrono::{Duration, Utc};
use hyperliquid_backtester::prelude::*;
use rs_backtester::prelude::*;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tokio::time::sleep;

/// # Advanced Portfolio Management Example
///
/// This example demonstrates sophisticated portfolio management techniques for trading
/// across multiple assets on Hyperliquid, including:
///
/// - Dynamic asset allocation based on volatility and correlation
/// - Portfolio rebalancing with custom schedules and thresholds
/// - Risk-adjusted position sizing across multiple assets
/// - Cross-asset correlation management
/// - Sector-based portfolio construction
/// - Performance attribution by asset class

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PortfolioAllocation {
    symbol: String,
    target_weight: f64,
    current_weight: f64,
    volatility_contribution: f64,
    correlation_score: f64,
    max_position_size: f64,
    funding_rate_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssetClass {
    name: String,
    target_allocation: f64,
    current_allocation: f64,
    assets: Vec<String>,
    performance: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Advanced Portfolio Management Example");
    println!("====================================\n");

    // Define asset universe with asset classes
    let mut asset_classes = HashMap::new();
    
    // Layer 1 assets
    asset_classes.insert("Layer1", AssetClass {
        name: "Layer1".to_string(),
        target_allocation: 0.40, // 40% allocation to Layer 1
        current_allocation: 0.0,
        assets: vec!["BTC".to_string(), "ETH".to_string()],
        performance: 0.0,
    });
    
    // Layer 2 assets
    asset_classes.insert("Layer2", AssetClass {
        name: "Layer2".to_string(),
        target_allocation: 0.30, // 30% allocation to Layer 2
        current_allocation: 0.0,
        assets: vec!["SOL".to_string(), "AVAX".to_string()],
        performance: 0.0,
    });
    
    // DeFi assets
    asset_classes.insert("DeFi", AssetClass {
        name: "DeFi".to_string(),
        target_allocation: 0.20, // 20% allocation to DeFi
        current_allocation: 0.0,
        assets: vec!["UNI".to_string(), "AAVE".to_string()],
        performance: 0.0,
    });
    
    // Meme assets (smaller allocation due to higher risk)
    asset_classes.insert("Meme", AssetClass {
        name: "Meme".to_string(),
        target_allocation: 0.10, // 10% allocation to Meme coins
        current_allocation: 0.0,
        assets: vec!["DOGE".to_string(), "SHIB".to_string()],
        performance: 0.0,
    });
    
    // Collect all assets from all classes
    let mut all_assets = Vec::new();
    for asset_class in asset_classes.values() {
        for asset in &asset_class.assets {
            all_assets.push(asset.clone());
        }
    }
    
    println!("Asset universe: {:?}", all_assets);
    
    // Fetch historical data for all assets
    let end_time = Utc::now().timestamp() as u64;
    let start_time = end_time - (90 * 24 * 3600); // 90 days of data
    
    println!("Fetching historical data for {} assets...", all_assets.len());
    
    let mut asset_data = HashMap::new();
    for asset in &all_assets {
        println!("Fetching {} data...", asset);
        
        // For this example, we'll use mock data for assets other than BTC and ETH
        let data = match asset.as_str() {
            "BTC" => HyperliquidData::fetch_btc("1h", start_time, end_time).await?,
            "ETH" => HyperliquidData::fetch_eth("1h", start_time, end_time).await?,
            _ => {
                println!("Using mock data for {}", asset);
                // In a real implementation, you would fetch actual data
                // For this example, we'll clone BTC data and modify it slightly
                let mut btc_data = HyperliquidData::fetch_btc("1h", start_time, end_time).await?;
                
                // Modify the data slightly to simulate different asset behavior
                for i in 0..btc_data.close.len() {
                    let random_factor = 0.8 + (asset.len() as f64 * 0.1);
                    btc_data.close[i] *= random_factor;
                    btc_data.open[i] *= random_factor;
                    btc_data.high[i] *= random_factor;
                    btc_data.low[i] *= random_factor;
                }
                
                btc_data
            }
        };
        
        println!("  {} data points fetched", data.len());
        asset_data.insert(asset.clone(), data);
    }
    
    // Calculate volatility for each asset
    println!("\nCalculating asset volatilities...");
    let mut asset_volatilities = HashMap::new();
    for (symbol, data) in &asset_data {
        let volatility = calculate_volatility(&data.close);
        asset_volatilities.insert(symbol.clone(), volatility);
        println!("  {}: {:.4}%", symbol, volatility * 100.0);
    }
    
    // Calculate correlation matrix
    println!("\nCalculating correlation matrix...");
    let correlation_matrix = calculate_correlation_matrix(&asset_data);
    
    // Print correlation matrix
    println!("\nCorrelation Matrix:");
    print!("{:10}", "");
    for asset in &all_assets {
        print!("{:10}", asset);
    }
    println!();
    
    for asset1 in &all_assets {
        print!("{:10}", asset1);
        for asset2 in &all_assets {
            let key = format!("{}-{}", asset1, asset2);
            let correlation = correlation_matrix.get(&key).unwrap_or(&1.0);
            print!("{:10.4}", correlation);
        }
        println!();
    }
    
    // Calculate funding rates for each asset
    println!("\nAnalyzing funding rates...");
    let mut funding_rate_scores = HashMap::new();
    for (symbol, data) in &asset_data {
        let avg_funding_rate = calculate_average_funding_rate(data);
        funding_rate_scores.insert(symbol.clone(), avg_funding_rate);
        println!("  {}: {:.6}% per 8h", symbol, avg_funding_rate * 100.0);
    }
    
    // Calculate optimal portfolio weights using risk parity approach
    println!("\nCalculating optimal portfolio weights using risk parity...");
    let portfolio_weights = calculate_risk_parity_weights(
        &all_assets,
        &asset_volatilities,
        &correlation_matrix,
        &funding_rate_scores
    );
    
    // Print optimal weights
    println!("\nOptimal Portfolio Weights:");
    for (symbol, weight) in &portfolio_weights {
        println!("  {}: {:.2}%", symbol, weight * 100.0);
    }
    
    // Adjust weights to respect asset class allocations
    println!("\nAdjusting weights to respect asset class allocations...");
    let adjusted_weights = adjust_weights_by_asset_class(
        &portfolio_weights,
        &asset_classes
    );
    
    // Print adjusted weights
    println!("\nAdjusted Portfolio Weights:");
    for (symbol, weight) in &adjusted_weights {
        println!("  {}: {:.2}%", symbol, weight * 100.0);
    }
    
    // Calculate maximum position sizes based on risk limits
    println!("\nCalculating maximum position sizes...");
    let max_position_sizes = calculate_max_position_sizes(
        &all_assets,
        &asset_volatilities,
        &correlation_matrix,
        100000.0, // Initial capital
        0.02      // 2% max daily VaR
    );
    
    // Print maximum position sizes
    println!("\nMaximum Position Sizes:");
    for (symbol, max_size) in &max_position_sizes {
        println!("  {}: ${:.2}", symbol, max_size);
    }
    
    // Create portfolio allocations
    let mut portfolio_allocations = Vec::new();
    for asset in &all_assets {
        portfolio_allocations.push(PortfolioAllocation {
            symbol: asset.clone(),
            target_weight: *adjusted_weights.get(asset).unwrap_or(&0.0),
            current_weight: 0.0, // Will be set during rebalancing
            volatility_contribution: 0.0, // Will be calculated
            correlation_score: calculate_correlation_score(asset, &all_assets, &correlation_matrix),
            max_position_size: *max_position_sizes.get(asset).unwrap_or(&0.0),
            funding_rate_score: *funding_rate_scores.get(asset).unwrap_or(&0.0),
        });
    }
    
    // Simulate portfolio rebalancing
    println!("\nSimulating portfolio rebalancing...");
    let rebalanced_portfolio = simulate_portfolio_rebalancing(
        &portfolio_allocations,
        &asset_data,
        100000.0, // Initial capital
        30,       // Rebalance every 30 days
        0.05      // 5% threshold for rebalancing
    ).await?;
    
    // Export portfolio allocation to CSV
    export_portfolio_allocation(&portfolio_allocations)?;
    
    // Export rebalancing results to CSV
    export_rebalancing_results(&rebalanced_portfolio)?;
    
    println!("\nPortfolio management example completed successfully!");
    println!("Portfolio allocation exported to portfolio_allocation.csv");
    println!("Rebalancing results exported to portfolio_rebalancing.csv");
    
    Ok(())
}

fn calculate_volatility(prices: &[f64]) -> f64 {
    if prices.len() < 2 {
        return 0.0;
    }
    
    // Calculate daily returns
    let mut returns = Vec::with_capacity(prices.len() - 1);
    for i in 1..prices.len() {
        let daily_return = (prices[i] - prices[i - 1]) / prices[i - 1];
        returns.push(daily_return);
    }
    
    // Calculate standard deviation of returns
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns.iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>() / (returns.len() - 1) as f64;
    
    variance.sqrt()
}

fn calculate_correlation_matrix(asset_data: &HashMap<String, HyperliquidData>) -> HashMap<String, f64> {
    let mut correlation_matrix = HashMap::new();
    let assets: Vec<String> = asset_data.keys().cloned().collect();
    
    for i in 0..assets.len() {
        let asset1 = &assets[i];
        let data1 = &asset_data[asset1];
        
        // Calculate returns for asset1
        let returns1: Vec<f64> = (1..data1.close.len())
            .map(|j| (data1.close[j] - data1.close[j - 1]) / data1.close[j - 1])
            .collect();
        
        for j in 0..assets.len() {
            let asset2 = &assets[j];
            let key = format!("{}-{}", asset1, asset2);
            
            if i == j {
                correlation_matrix.insert(key, 1.0);
                continue;
            }
            
            let data2 = &asset_data[asset2];
            
            // Calculate returns for asset2
            let returns2: Vec<f64> = (1..data2.close.len())
                .map(|j| (data2.close[j] - data2.close[j - 1]) / data2.close[j - 1])
                .collect();
            
            // Calculate correlation
            let min_len = returns1.len().min(returns2.len());
            let correlation = calculate_correlation(&returns1[..min_len], &returns2[..min_len]);
            
            correlation_matrix.insert(key, correlation);
        }
    }
    
    correlation_matrix
}

fn calculate_correlation(x: &[f64], y: &[f64]) -> f64 {
    if x.len() != y.len() || x.len() < 2 {
        return 0.0;
    }
    
    let n = x.len() as f64;
    let mean_x = x.iter().sum::<f64>() / n;
    let mean_y = y.iter().sum::<f64>() / n;
    
    let numerator: f64 = x.iter().zip(y.iter())
        .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
        .sum();
    
    let sum_sq_x: f64 = x.iter().map(|xi| (xi - mean_x).powi(2)).sum();
    let sum_sq_y: f64 = y.iter().map(|yi| (yi - mean_y).powi(2)).sum();
    
    let denominator = (sum_sq_x * sum_sq_y).sqrt();
    
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

fn calculate_average_funding_rate(data: &HyperliquidData) -> f64 {
    let mut sum = 0.0;
    let mut count = 0;
    
    for rate in &data.funding_rates {
        if !rate.is_nan() {
            sum += *rate;
            count += 1;
        }
    }
    
    if count > 0 {
        sum / count as f64
    } else {
        0.0
    }
}

fn calculate_risk_parity_weights(
    assets: &[String],
    volatilities: &HashMap<String, f64>,
    correlation_matrix: &HashMap<String, f64>,
    funding_rate_scores: &HashMap<String, f64>
) -> HashMap<String, f64> {
    let mut weights = HashMap::new();
    
    // Calculate inverse volatility weights as a starting point
    let mut total_inv_vol = 0.0;
    for asset in assets {
        let vol = *volatilities.get(asset).unwrap_or(&0.01);
        let inv_vol = 1.0 / vol.max(0.001);
        total_inv_vol += inv_vol;
    }
    
    for asset in assets {
        let vol = *volatilities.get(asset).unwrap_or(&0.01);
        let inv_vol = 1.0 / vol.max(0.001);
        let weight = inv_vol / total_inv_vol;
        weights.insert(asset.clone(), weight);
    }
    
    // Adjust weights based on correlation
    let mut adjusted_weights = HashMap::new();
    let mut total_weight = 0.0;
    
    for asset in assets {
        let base_weight = *weights.get(asset).unwrap_or(&0.0);
        let correlation_score = calculate_correlation_score(asset, assets, correlation_matrix);
        
        // Penalize highly correlated assets
        let correlation_factor = 1.0 - (correlation_score * 0.5);
        
        // Consider funding rate - boost assets with positive funding rates
        let funding_rate = *funding_rate_scores.get(asset).unwrap_or(&0.0);
        let funding_factor = if funding_rate > 0.0 {
            1.0 + (funding_rate * 10.0).min(0.5)
        } else {
            1.0 + (funding_rate * 5.0).max(-0.3)
        };
        
        let adjusted_weight = base_weight * correlation_factor * funding_factor;
        adjusted_weights.insert(asset.clone(), adjusted_weight);
        total_weight += adjusted_weight;
    }
    
    // Normalize weights
    for (_, weight) in adjusted_weights.iter_mut() {
        *weight /= total_weight;
    }
    
    adjusted_weights
}

fn calculate_correlation_score(
    asset: &str,
    all_assets: &[String],
    correlation_matrix: &HashMap<String, f64>
) -> f64 {
    let mut total_correlation = 0.0;
    let mut count = 0;
    
    for other_asset in all_assets {
        if asset == other_asset {
            continue;
        }
        
        let key = format!("{}-{}", asset, other_asset);
        if let Some(correlation) = correlation_matrix.get(&key) {
            total_correlation += correlation.abs();
            count += 1;
        }
    }
    
    if count > 0 {
        total_correlation / count as f64
    } else {
        0.0
    }
}

fn adjust_weights_by_asset_class(
    weights: &HashMap<String, f64>,
    asset_classes: &HashMap<&str, AssetClass>
) -> HashMap<String, f64> {
    let mut adjusted_weights = HashMap::new();
    
    // Calculate current allocation by asset class
    let mut class_allocations = HashMap::new();
    for (asset, weight) in weights {
        for (class_name, asset_class) in asset_classes {
            if asset_class.assets.contains(asset) {
                let current = class_allocations.get(class_name).unwrap_or(&0.0);
                class_allocations.insert(*class_name, current + weight);
                break;
            }
        }
    }
    
    // Calculate adjustment factors for each asset class
    let mut class_factors = HashMap::new();
    for (class_name, asset_class) in asset_classes {
        let current_allocation = *class_allocations.get(class_name).unwrap_or(&0.0);
        if current_allocation > 0.0 {
            let factor = asset_class.target_allocation / current_allocation;
            class_factors.insert(*class_name, factor);
        } else {
            class_factors.insert(*class_name, 1.0);
        }
    }
    
    // Apply adjustment factors to individual assets
    let mut total_adjusted_weight = 0.0;
    for (asset, weight) in weights {
        for (class_name, asset_class) in asset_classes {
            if asset_class.assets.contains(asset) {
                let factor = *class_factors.get(class_name).unwrap_or(&1.0);
                let adjusted_weight = weight * factor;
                adjusted_weights.insert(asset.clone(), adjusted_weight);
                total_adjusted_weight += adjusted_weight;
                break;
            }
        }
    }
    
    // Normalize weights
    for (_, weight) in adjusted_weights.iter_mut() {
        *weight /= total_adjusted_weight;
    }
    
    adjusted_weights
}

fn calculate_max_position_sizes(
    assets: &[String],
    volatilities: &HashMap<String, f64>,
    correlation_matrix: &HashMap<String, f64>,
    portfolio_value: f64,
    max_var: f64
) -> HashMap<String, f64> {
    let mut max_sizes = HashMap::new();
    
    for asset in assets {
        let vol = *volatilities.get(asset).unwrap_or(&0.01);
        
        // Calculate average correlation with other assets
        let avg_correlation = calculate_correlation_score(asset, assets, correlation_matrix);
        
        // Higher correlation means lower position size
        let correlation_factor = 1.0 - (avg_correlation * 0.5);
        
        // Calculate maximum position size based on volatility and correlation
        // Using a simplified VaR calculation: VaR = Position * Volatility * 1.65 (95% confidence)
        let max_position = (max_var * portfolio_value) / (vol * 1.65);
        
        // Apply correlation adjustment
        let adjusted_max_position = max_position * correlation_factor;
        
        // Cap at 20% of portfolio value
        let capped_position = adjusted_max_position.min(portfolio_value * 0.2);
        
        max_sizes.insert(asset.clone(), capped_position);
    }
    
    max_sizes
}

async fn simulate_portfolio_rebalancing(
    allocations: &[PortfolioAllocation],
    asset_data: &HashMap<String, HyperliquidData>,
    initial_capital: f64,
    rebalance_days: usize,
    rebalance_threshold: f64
) -> Result<Vec<HashMap<String, f64>>> {
    let mut portfolio_value = initial_capital;
    let mut asset_values = HashMap::new();
    let mut rebalancing_history = Vec::new();
    
    // Initialize asset values based on target weights
    for allocation in allocations {
        let asset_value = initial_capital * allocation.target_weight;
        asset_values.insert(allocation.symbol.clone(), asset_value);
    }
    
    // Record initial allocation
    rebalancing_history.push(asset_values.clone());
    
    // Find the shortest data series to align all assets
    let min_length = asset_data.values().map(|data| data.len()).min().unwrap_or(0);
    
    // Convert rebalance_days to hours (assuming hourly data)
    let rebalance_frequency = rebalance_days * 24;
    let mut last_rebalance = 0;
    
    for i in 1..min_length {
        // Calculate current portfolio value
        portfolio_value = 0.0;
        
        for (symbol, data) in asset_data {
            if i >= data.close.len() {
                continue;
            }
            
            let prev_price = data.close[i - 1];
            let curr_price = data.close[i];
            let return_rate = (curr_price - prev_price) / prev_price;
            
            // Update asset value
            if let Some(value) = asset_values.get_mut(symbol) {
                *value *= (1.0 + return_rate);
                portfolio_value += *value;
            }
        }
        
        // Check if rebalancing is needed
        let should_rebalance = i - last_rebalance >= rebalance_frequency;
        
        if should_rebalance {
            println!("Scheduled rebalancing at hour {}", i);
            
            // Calculate current weights
            let mut current_weights = HashMap::new();
            for (symbol, value) in &asset_values {
                current_weights.insert(symbol.clone(), value / portfolio_value);
            }
            
            // Check if any weight deviates from target by more than the threshold
            let mut max_deviation = 0.0;
            for allocation in allocations {
                let current_weight = *current_weights.get(&allocation.symbol).unwrap_or(&0.0);
                let deviation = (current_weight - allocation.target_weight).abs();
                max_deviation = max_deviation.max(deviation);
            }
            
            if max_deviation > rebalance_threshold {
                println!("  Rebalancing triggered: max deviation {:.2}% exceeds threshold {:.2}%", 
                         max_deviation * 100.0, rebalance_threshold * 100.0);
                
                // Rebalance to target weights
                for allocation in allocations {
                    let target_value = portfolio_value * allocation.target_weight;
                    asset_values.insert(allocation.symbol.clone(), target_value);
                }
                
                // Record rebalanced allocation
                rebalancing_history.push(asset_values.clone());
            } else {
                println!("  No rebalancing needed: max deviation {:.2}% below threshold {:.2}%", 
                         max_deviation * 100.0, rebalance_threshold * 100.0);
            }
            
            last_rebalance = i;
        }
    }
    
    Ok(rebalancing_history)
}

fn export_portfolio_allocation(allocations: &[PortfolioAllocation]) -> Result<()> {
    let mut csv = String::from("symbol,target_weight,volatility_contribution,correlation_score,max_position_size,funding_rate_score\n");
    
    for allocation in allocations {
        csv.push_str(&format!(
            "{},{:.4},{:.4},{:.4},{:.2},{:.6}\n",
            allocation.symbol,
            allocation.target_weight,
            allocation.volatility_contribution,
            allocation.correlation_score,
            allocation.max_position_size,
            allocation.funding_rate_score
        ));
    }
    
    let mut file = File::create("portfolio_allocation.csv")?;
    file.write_all(csv.as_bytes())?;
    
    Ok(())
}

fn export_rebalancing_results(history: &[HashMap<String, f64>]) -> Result<()> {
    if history.is_empty() {
        return Ok(());
    }
    
    // Get all symbols
    let mut symbols = Vec::new();
    for (symbol, _) in &history[0] {
        symbols.push(symbol.clone());
    }
    
    // Create CSV header
    let mut csv = String::from("rebalance_id");
    for symbol in &symbols {
        csv.push_str(&format!(",{}", symbol));
    }
    csv.push('\n');
    
    // Add data rows
    for (i, allocation) in history.iter().enumerate() {
        csv.push_str(&format!("{}", i));
        
        for symbol in &symbols {
            let value = allocation.get(symbol).unwrap_or(&0.0);
            csv.push_str(&format!(",{:.2}", value));
        }
        
        csv.push('\n');
    }
    
    let mut file = File::create("portfolio_rebalancing.csv")?;
    file.write_all(csv.as_bytes())?;
    
    Ok(())
}