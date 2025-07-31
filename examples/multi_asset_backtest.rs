use chrono::{Duration, Utc};
use hyperliquid_backtest::prelude::*;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// # Multi-Asset Backtesting Example
///
/// This example demonstrates how to backtest strategies across multiple assets
/// simultaneously, including:
/// - Portfolio-level risk management
/// - Cross-asset correlation analysis
/// - Dynamic asset allocation based on market conditions
/// - Comprehensive multi-asset performance reporting
/// - Sector rotation strategies

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssetAllocation {
    symbol: String,
    target_weight: f64,
    current_weight: f64,
    performance: f64,
    volatility: f64,
    sharpe_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PortfolioMetrics {
    timestamp: i64,
    total_value: f64,
    btc_value: f64,
    eth_value: f64,
    sol_value: f64,
    avax_value: f64,
    portfolio_beta: f64,
    diversification_ratio: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Multi-Asset Backtesting Example");
    println!("==============================\n");

    // Define asset universe
    let assets = vec!["BTC", "ETH", "SOL", "AVAX"];
    let mut asset_data = HashMap::new();
    
    // Fetch historical data for all assets
    let end_time = Utc::now();
    let start_time = end_time - Duration::days(30);
    let start_timestamp = start_time.timestamp_millis() as u64;
    let end_timestamp = end_time.timestamp_millis() as u64;
    
    println!("Time range debug:");
    println!("  Current time: {}", end_time);
    println!("  Start time: {}", start_time);
    println!("  Start timestamp: {}", start_timestamp);
    println!("  End timestamp: {}", end_timestamp);
    println!();
    
    println!("Fetching historical data for {} assets...", assets.len());
    
    for asset in &assets {
        println!("Fetching {} data...", asset);
        let data = HyperliquidData::fetch(asset, "1h", start_timestamp, end_timestamp).await?;
        
        println!("  {} data points fetched", data.len());
        asset_data.insert(asset.to_string(), data);
    }
    
    // Strategy 1: Equal Weight Portfolio Rebalancing
    println!("\nStrategy 1: Equal Weight Portfolio with Monthly Rebalancing");
    
    let equal_weight_results = run_equal_weight_strategy(&asset_data, 25000.0).await?;
    
    // Strategy 2: Momentum-Based Asset Rotation
    println!("\nStrategy 2: Momentum-Based Asset Rotation");
    
    let momentum_results = run_momentum_rotation_strategy(&asset_data, 25000.0).await?;
    
    // Strategy 3: Volatility-Adjusted Portfolio
    println!("\nStrategy 3: Risk Parity (Volatility-Adjusted) Portfolio");
    
    let risk_parity_results = run_risk_parity_strategy(&asset_data, 25000.0).await?;
    
    // Strategy 4: Funding Rate Arbitrage Across Assets
    println!("\nStrategy 4: Cross-Asset Funding Rate Arbitrage");
    
    let funding_arb_results = run_cross_asset_funding_strategy(&asset_data, 25000.0).await?;
    
    // Compare all strategies
    println!("\nMulti-Asset Strategy Comparison:");
    println!("===============================");
    
    let strategies = vec![
        ("Equal Weight", equal_weight_results),
        ("Momentum Rotation", momentum_results),
        ("Risk Parity", risk_parity_results),
        ("Funding Arbitrage", funding_arb_results),
    ];
    
    println!("{:<20} {:<15} {:<15} {:<15} {:<15}", 
        "Strategy", "Final Value", "Total Return", "Max DD", "Sharpe");
    println!("{}", "-".repeat(80));
    
    for (name, results) in &strategies {
        println!("{:<20} ${:<14.2} {:<14.2}% {:<14.2}% {:<14.3}", 
            name,
            results.final_equity,
            results.total_return * 100.0,
            results.max_drawdown * 100.0,
            results.sharpe_ratio);
    }
    
    // Export comprehensive results
    export_multi_asset_results(&strategies, &asset_data).await?;
    
    // Portfolio correlation analysis
    analyze_portfolio_correlations(&asset_data).await?;
    
    println!("\nMulti-asset backtesting example completed successfully!");
    
    Ok(())
}

#[derive(Debug, Clone)]
struct StrategyResults {
    final_equity: f64,
    total_return: f64,
    max_drawdown: f64,
    sharpe_ratio: f64,
    volatility: f64,
    equity_curve: Vec<f64>,
    timestamps: Vec<i64>,
    allocations: Vec<AssetAllocation>,
}

async fn run_equal_weight_strategy(
    asset_data: &HashMap<String, HyperliquidData>,
    initial_capital: f64,
) -> Result<StrategyResults> {
    println!("Running equal weight rebalancing strategy...");
    
    // Get the shortest data series to align all assets
    let min_length = asset_data.values().map(|data| data.len()).min().unwrap_or(0);
    let target_weight = 1.0 / asset_data.len() as f64; // Equal weight
    
    let mut portfolio_value = initial_capital;
    let mut equity_curve = Vec::new();
    let mut timestamps = Vec::new();
    let mut max_drawdown = 0.0;
    let mut peak_value = initial_capital;
    
    // Rebalance every 30 days (720 hours)
    let rebalance_frequency = 720;
    let mut last_rebalance = 0;
    
    // Track individual asset allocations
    let mut asset_values: HashMap<String, f64> = asset_data.keys()
        .map(|k| (k.clone(), initial_capital * target_weight))
        .collect();
    
    for i in 1..min_length {
        // Calculate current portfolio value
        portfolio_value = 0.0;
        
        for (symbol, data) in asset_data {
            let prev_price = data.close[i - 1];
            let curr_price = data.close[i];
            let return_rate = (curr_price - prev_price) / prev_price;
            
            // Update asset value
            let current_value = asset_values[symbol] * (1.0 + return_rate);
            asset_values.insert(symbol.clone(), current_value);
            portfolio_value += current_value;
        }
        
        // Rebalancing logic
        if i - last_rebalance >= rebalance_frequency {
            println!("Rebalancing at index {} (portfolio value: ${:.2})", i, portfolio_value);
            
            // Rebalance to equal weights
            for symbol in asset_data.keys() {
                asset_values.insert(symbol.clone(), portfolio_value * target_weight);
            }
            
            last_rebalance = i;
        }
        
        // Track drawdown
        if portfolio_value > peak_value {
            peak_value = portfolio_value;
        }
        let current_drawdown = (peak_value - portfolio_value) / peak_value;
        if current_drawdown > max_drawdown {
            max_drawdown = current_drawdown;
        }
        
        // Record equity curve (every 24 hours)
        if i % 24 == 0 {
            equity_curve.push(portfolio_value);
            timestamps.push(asset_data.values().next().unwrap().datetime[i].timestamp());
        }
    }
    
    let total_return = (portfolio_value - initial_capital) / initial_capital;
    let volatility = calculate_volatility(&equity_curve);
    let sharpe_ratio = if volatility > 0.0 {
        (total_return * 365.25) / (volatility * (365.25_f64).sqrt())
    } else {
        0.0
    };
    
    // Create final allocations
    let allocations = asset_data.keys().map(|symbol| {
        AssetAllocation {
            symbol: symbol.clone(),
            target_weight,
            current_weight: asset_values[symbol] / portfolio_value,
            performance: (asset_values[symbol] - initial_capital * target_weight) / (initial_capital * target_weight),
            volatility: 0.0, // Would need individual asset volatility calculation
            sharpe_ratio: 0.0, // Would need individual asset Sharpe calculation
        }
    }).collect();
    
    Ok(StrategyResults {
        final_equity: portfolio_value,
        total_return,
        max_drawdown,
        sharpe_ratio,
        volatility,
        equity_curve,
        timestamps,
        allocations,
    })
}

async fn run_momentum_rotation_strategy(
    asset_data: &HashMap<String, HyperliquidData>,
    initial_capital: f64,
) -> Result<StrategyResults> {
    println!("Running momentum rotation strategy...");
    
    let min_length = asset_data.values().map(|data| data.len()).min().unwrap_or(0);
    let lookback_period = 168; // 7 days (168 hours)
    let rotation_frequency = 168; // Rotate weekly
    
    let mut portfolio_value = initial_capital;
    let mut equity_curve = Vec::new();
    let mut timestamps = Vec::new();
    let mut max_drawdown = 0.0;
    let mut peak_value = initial_capital;
    let mut current_asset = "BTC".to_string();
    let mut last_rotation = 0;
    
    for i in lookback_period..min_length {
        // Calculate momentum for all assets
        if i - last_rotation >= rotation_frequency {
            let mut momentum_scores = HashMap::new();
            
            for (symbol, data) in asset_data {
                let current_price = data.close[i];
                let past_price = data.close[i - lookback_period];
                let momentum = (current_price - past_price) / past_price;
                momentum_scores.insert(symbol.clone(), momentum);
            }
            
            // Select asset with highest momentum
            let best_asset = momentum_scores.iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(k, _)| k.clone())
                .unwrap_or_else(|| "BTC".to_string());
            
            if best_asset != current_asset {
                println!("Rotating from {} to {} at index {} (momentum: {:.4})", 
                    current_asset, best_asset, i, momentum_scores[&best_asset]);
                current_asset = best_asset;
            }
            
            last_rotation = i;
        }
        
        // Calculate return for current asset
        let data = &asset_data[&current_asset];
        let prev_price = data.close[i - 1];
        let curr_price = data.close[i];
        let return_rate = (curr_price - prev_price) / prev_price;
        
        portfolio_value *= 1.0 + return_rate;
        
        // Track drawdown
        if portfolio_value > peak_value {
            peak_value = portfolio_value;
        }
        let current_drawdown = (peak_value - portfolio_value) / peak_value;
        if current_drawdown > max_drawdown {
            max_drawdown = current_drawdown;
        }
        
        // Record equity curve
        if i % 24 == 0 {
            equity_curve.push(portfolio_value);
            timestamps.push(asset_data.values().next().unwrap().datetime[i].timestamp());
        }
    }
    
    let total_return = (portfolio_value - initial_capital) / initial_capital;
    let volatility = calculate_volatility(&equity_curve);
    let sharpe_ratio = if volatility > 0.0 {
        (total_return * 365.25) / (volatility * (365.25_f64).sqrt())
    } else {
        0.0
    };
    
    Ok(StrategyResults {
        final_equity: portfolio_value,
        total_return,
        max_drawdown,
        sharpe_ratio,
        volatility,
        equity_curve,
        timestamps,
        allocations: vec![], // Simplified for momentum strategy
    })
}

async fn run_risk_parity_strategy(
    asset_data: &HashMap<String, HyperliquidData>,
    initial_capital: f64,
) -> Result<StrategyResults> {
    println!("Running risk parity strategy...");
    
    let min_length = asset_data.values().map(|data| data.len()).min().unwrap_or(0);
    let volatility_window = 720; // 30 days
    let rebalance_frequency = 168; // Weekly rebalancing
    
    let mut portfolio_value = initial_capital;
    let mut equity_curve = Vec::new();
    let mut timestamps = Vec::new();
    let mut max_drawdown = 0.0;
    let mut peak_value = initial_capital;
    let mut last_rebalance = volatility_window;
    
    // Initialize equal weights
    let mut asset_weights: HashMap<String, f64> = asset_data.keys()
        .map(|k| (k.clone(), 1.0 / asset_data.len() as f64))
        .collect();
    
    let mut asset_values: HashMap<String, f64> = asset_data.keys()
        .map(|k| (k.clone(), initial_capital * asset_weights[k]))
        .collect();
    
    for i in volatility_window..min_length {
        // Calculate portfolio value
        portfolio_value = 0.0;
        
        for (symbol, data) in asset_data {
            let prev_price = data.close[i - 1];
            let curr_price = data.close[i];
            let return_rate = (curr_price - prev_price) / prev_price;
            
            let current_value = asset_values[symbol] * (1.0 + return_rate);
            asset_values.insert(symbol.clone(), current_value);
            portfolio_value += current_value;
        }
        
        // Rebalancing based on inverse volatility
        if i - last_rebalance >= rebalance_frequency {
            let mut volatilities = HashMap::new();
            
            // Calculate volatilities
            for (symbol, data) in asset_data {
                let returns: Vec<f64> = (i - volatility_window + 1..=i)
                    .map(|j| {
                        let prev = data.close[j - 1];
                        let curr = data.close[j];
                        (curr - prev) / prev
                    })
                    .collect();
                
                let volatility = calculate_volatility(&returns);
                volatilities.insert(symbol.clone(), volatility);
            }
            
            // Calculate inverse volatility weights
            let total_inv_vol: f64 = volatilities.values().map(|v| 1.0 / v.max(0.001)).sum();
            
            for symbol in asset_data.keys() {
                let inv_vol = 1.0 / volatilities[symbol].max(0.001);
                let weight = inv_vol / total_inv_vol;
                asset_weights.insert(symbol.clone(), weight);
                asset_values.insert(symbol.clone(), portfolio_value * weight);
            }
            
            println!("Risk parity rebalancing at index {}", i);
            for (symbol, weight) in &asset_weights {
                println!("  {}: {:.2}% (vol: {:.4})", symbol, weight * 100.0, volatilities[symbol]);
            }
            
            last_rebalance = i;
        }
        
        // Track drawdown
        if portfolio_value > peak_value {
            peak_value = portfolio_value;
        }
        let current_drawdown = (peak_value - portfolio_value) / peak_value;
        if current_drawdown > max_drawdown {
            max_drawdown = current_drawdown;
        }
        
        // Record equity curve
        if i % 24 == 0 {
            equity_curve.push(portfolio_value);
            timestamps.push(asset_data.values().next().unwrap().datetime[i].timestamp());
        }
    }
    
    let total_return = (portfolio_value - initial_capital) / initial_capital;
    let volatility = calculate_volatility(&equity_curve);
    let sharpe_ratio = if volatility > 0.0 {
        (total_return * 365.25) / (volatility * (365.25_f64).sqrt())
    } else {
        0.0
    };
    
    Ok(StrategyResults {
        final_equity: portfolio_value,
        total_return,
        max_drawdown,
        sharpe_ratio,
        volatility,
        equity_curve,
        timestamps,
        allocations: vec![], // Simplified
    })
}

async fn run_cross_asset_funding_strategy(
    asset_data: &HashMap<String, HyperliquidData>,
    initial_capital: f64,
) -> Result<StrategyResults> {
    println!("Running cross-asset funding arbitrage strategy...");
    
    let min_length = asset_data.values().map(|data| data.len()).min().unwrap_or(0);
    let funding_threshold = 0.0005; // 0.05% per 8h
    
    let mut portfolio_value = initial_capital;
    let mut equity_curve = Vec::new();
    let mut timestamps = Vec::new();
    let mut max_drawdown = 0.0;
    let mut peak_value = initial_capital;
    
    // Track positions in each asset
    let mut asset_positions: HashMap<String, f64> = asset_data.keys()
        .map(|k| (k.clone(), 0.0))
        .collect();
    
    let mut cash = initial_capital;
    
    for i in 1..min_length {
        // Update portfolio value based on price changes
        portfolio_value = cash;
        
        for (symbol, data) in asset_data {
            if asset_positions[symbol] != 0.0 {
                let prev_price = data.close[i - 1];
                let curr_price = data.close[i];
                let price_change = curr_price - prev_price;
                let position_pnl = asset_positions[symbol] * price_change;
                
                // Add funding payments (simplified)
                let funding_rate = if i < data.funding_rates.len() && !data.funding_rates[i].is_nan() {
                    data.funding_rates[i]
                } else {
                    0.0
                };
                
                let funding_payment = if asset_positions[symbol] > 0.0 {
                    // Long position receives funding if rate is positive
                    asset_positions[symbol] * curr_price * funding_rate / (3.0 * 365.25) // Approximate daily funding
                } else if asset_positions[symbol] < 0.0 {
                    // Short position pays funding if rate is positive
                    asset_positions[symbol] * curr_price * funding_rate / (3.0 * 365.25)
                } else {
                    0.0
                };
                
                portfolio_value += position_pnl + funding_payment;
            }
        }
        
        // Funding arbitrage logic
        for (symbol, data) in asset_data {
            if i >= data.funding_rates.len() {
                continue;
            }
            
            let funding_rate = data.funding_rates[i];
            if funding_rate.is_nan() {
                continue;
            }
            
            let current_price = data.close[i];
            let current_position = asset_positions[symbol];
            let position_value = current_position.abs() * current_price;
            
            // Close positions if funding becomes unfavorable
            if current_position > 0.0 && funding_rate < -funding_threshold {
                // Long position but negative funding - close
                cash += position_value;
                asset_positions.insert(symbol.clone(), 0.0);
                println!("Closed long {} position due to negative funding: {:.6}", symbol, funding_rate);
            } else if current_position < 0.0 && funding_rate > funding_threshold {
                // Short position but positive funding - close
                cash += position_value;
                asset_positions.insert(symbol.clone(), 0.0);
                println!("Closed short {} position due to positive funding: {:.6}", symbol, funding_rate);
            }
            
            // Open new positions based on funding opportunities
            if current_position == 0.0 && funding_rate.abs() > funding_threshold {
                let position_size = (portfolio_value * 0.2) / current_price; // 20% of portfolio per position
                
                if funding_rate > funding_threshold {
                    // Positive funding - go long to receive funding
                    asset_positions.insert(symbol.clone(), position_size);
                    cash -= position_size * current_price;
                    println!("Opened long {} position for funding: {:.6}", symbol, funding_rate);
                } else if funding_rate < -funding_threshold {
                    // Negative funding - go short to receive funding
                    asset_positions.insert(symbol.clone(), -position_size);
                    cash += position_size * current_price;
                    println!("Opened short {} position for funding: {:.6}", symbol, funding_rate);
                }
            }
        }
        
        // Track drawdown
        if portfolio_value > peak_value {
            peak_value = portfolio_value;
        }
        let current_drawdown = (peak_value - portfolio_value) / peak_value;
        if current_drawdown > max_drawdown {
            max_drawdown = current_drawdown;
        }
        
        // Record equity curve
        if i % 24 == 0 {
            equity_curve.push(portfolio_value);
            timestamps.push(asset_data.values().next().unwrap().datetime[i].timestamp());
        }
    }
    
    let total_return = (portfolio_value - initial_capital) / initial_capital;
    let volatility = calculate_volatility(&equity_curve);
    let sharpe_ratio = if volatility > 0.0 {
        (total_return * 365.25) / (volatility * (365.25_f64).sqrt())
    } else {
        0.0
    };
    
    Ok(StrategyResults {
        final_equity: portfolio_value,
        total_return,
        max_drawdown,
        sharpe_ratio,
        volatility,
        equity_curve,
        timestamps,
        allocations: vec![], // Simplified
    })
}

fn calculate_volatility(returns: &[f64]) -> f64 {
    if returns.len() < 2 {
        return 0.0;
    }
    
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns.iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>() / (returns.len() - 1) as f64;
    
    variance.sqrt()
}

async fn export_multi_asset_results(
    strategies: &[(&str, StrategyResults)],
    asset_data: &HashMap<String, HyperliquidData>,
) -> Result<()> {
    println!("\nExporting multi-asset results...");
    
    // Export strategy comparison
    let mut comparison_csv = String::from("strategy,final_equity,total_return,max_drawdown,sharpe_ratio,volatility\n");
    
    for (name, results) in strategies {
        comparison_csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            name, results.final_equity, results.total_return, 
            results.max_drawdown, results.sharpe_ratio, results.volatility
        ));
    }
    
    let mut file = File::create("multi_asset_comparison.csv")?;
    file.write_all(comparison_csv.as_bytes())?;
    println!("Strategy comparison exported to multi_asset_comparison.csv");
    
    // Export equity curves
    let mut equity_csv = String::from("timestamp");
    for (name, _) in strategies {
        equity_csv.push_str(&format!(",{}", name));
    }
    equity_csv.push('\n');
    
    // Find the minimum length of equity curves
    let min_length = strategies.iter()
        .map(|(_, results)| results.equity_curve.len())
        .min()
        .unwrap_or(0);
    
    for i in 0..min_length {
        if let Some((_, first_strategy)) = strategies.first() {
            if i < first_strategy.timestamps.len() {
                equity_csv.push_str(&first_strategy.timestamps[i].to_string());
                
                for (_, results) in strategies {
                    if i < results.equity_curve.len() {
                        equity_csv.push_str(&format!(",{}", results.equity_curve[i]));
                    } else {
                        equity_csv.push_str(",");
                    }
                }
                equity_csv.push('\n');
            }
        }
    }
    
    let mut file = File::create("multi_asset_equity_curves.csv")?;
    file.write_all(equity_csv.as_bytes())?;
    println!("Equity curves exported to multi_asset_equity_curves.csv");
    
    Ok(())
}

async fn analyze_portfolio_correlations(asset_data: &HashMap<String, HyperliquidData>) -> Result<()> {
    println!("\nAnalyzing portfolio correlations...");
    
    let min_length = asset_data.values().map(|data| data.len()).min().unwrap_or(0);
    let window_size = 720; // 30 days
    
    if min_length < window_size {
        println!("Insufficient data for correlation analysis");
        return Ok(());
    }
    
    // Calculate returns for each asset
    let mut asset_returns: HashMap<String, Vec<f64>> = HashMap::new();
    
    for (symbol, data) in asset_data {
        let returns: Vec<f64> = (1..min_length)
            .map(|i| {
                let prev = data.close[i - 1];
                let curr = data.close[i];
                (curr - prev) / prev
            })
            .collect();
        asset_returns.insert(symbol.clone(), returns);
    }
    
    // Calculate correlation matrix
    let assets: Vec<String> = asset_data.keys().cloned().collect();
    let mut correlation_csv = String::from("asset");
    for asset in &assets {
        correlation_csv.push_str(&format!(",{}", asset));
    }
    correlation_csv.push('\n');
    
    for asset1 in &assets {
        correlation_csv.push_str(asset1);
        
        for asset2 in &assets {
            let correlation = if asset1 == asset2 {
                1.0
            } else {
                calculate_correlation(&asset_returns[asset1], &asset_returns[asset2])
            };
            correlation_csv.push_str(&format!(",{:.4}", correlation));
        }
        correlation_csv.push('\n');
    }
    
    let mut file = File::create("asset_correlations.csv")?;
    file.write_all(correlation_csv.as_bytes())?;
    println!("Asset correlations exported to asset_correlations.csv");
    
    Ok(())
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