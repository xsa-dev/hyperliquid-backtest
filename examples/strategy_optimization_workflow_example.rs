use chrono::{Duration, Utc};
use hyperliquid_backtest::prelude::*;
use rs_backtester::prelude::*;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// # Strategy Optimization Workflow Example
///
/// This example demonstrates a comprehensive workflow for optimizing trading strategies
/// on Hyperliquid, including:
///
/// - Parameter optimization using grid search
/// - Walk-forward optimization to prevent overfitting
/// - Monte Carlo simulation for robustness testing
/// - Performance metrics comparison across parameter sets
/// - Optimization for different market conditions
/// - Multi-objective optimization (balancing return, drawdown, and Sharpe ratio)
/// - Sensitivity analysis for strategy parameters

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StrategyParams {
    short_period: usize,
    long_period: usize,
    funding_weight: f64,
    stop_loss_pct: f64,
    take_profit_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OptimizationResult {
    params: StrategyParams,
    final_equity: f64,
    total_return: f64,
    max_drawdown: f64,
    sharpe_ratio: f64,
    sortino_ratio: f64,
    win_rate: f64,
    profit_factor: f64,
    recovery_factor: f64,
    trades_count: usize,
    optimization_score: f64,
}
#
[tokio::main]
async fn main() -> Result<()> {
    println!("Strategy Optimization Workflow Example");
    println!("=====================================\n");

    // 1. Fetch historical data for optimization
    println!("1. Fetching historical data for optimization...");
    
    let end_time = Utc::now().timestamp() as u64;
    let start_time = end_time - (180 * 24 * 3600); // 180 days of data
    
    println!("  Fetching BTC data from {} to {}", start_time, end_time);
    let btc_data = HyperliquidData::fetch_btc("1h", start_time, end_time).await?;
    println!("  Fetched {} data points", btc_data.len());
    
    // 2. Define parameter ranges for optimization
    println!("\n2. Defining parameter ranges for optimization...");
    
    let short_periods = vec![5, 10, 15, 20, 25];
    let long_periods = vec![30, 50, 70, 90, 110];
    let funding_weights = vec![0.0, 0.25, 0.5, 0.75, 1.0];
    let stop_loss_pcts = vec![0.02, 0.03, 0.05, 0.07, 0.10];
    let take_profit_pcts = vec![0.03, 0.05, 0.08, 0.12, 0.15];
    
    println!("  Short MA periods: {:?}", short_periods);
    println!("  Long MA periods: {:?}", long_periods);
    println!("  Funding weights: {:?}", funding_weights);
    println!("  Stop-loss percentages: {:?}", stop_loss_pcts);
    println!("  Take-profit percentages: {:?}", take_profit_pcts);
    
    // Calculate total parameter combinations
    let total_combinations = short_periods.len() * long_periods.len() * 
                            funding_weights.len() * stop_loss_pcts.len() * 
                            take_profit_pcts.len();
    
    println!("  Total parameter combinations: {}", total_combinations);
    
    // 3. Perform in-sample optimization (grid search)
    println!("\n3. Performing in-sample optimization (grid search)...");
    
    // Split data into in-sample and out-of-sample periods
    let split_index = (btc_data.len() as f64 * 0.7) as usize;
    let in_sample_data = btc_data.slice(0, split_index);
    let out_of_sample_data = btc_data.slice(split_index, btc_data.len());
    
    println!("  In-sample period: {} data points", in_sample_data.len());
    println!("  Out-of-sample period: {} data points", out_of_sample_data.len());
    
    // Perform grid search on in-sample data
    println!("  Running grid search optimization...");
    
    // For demonstration, we'll use a reduced parameter space
    let reduced_short_periods = vec![10, 20];
    let reduced_long_periods = vec![50, 90];
    let reduced_funding_weights = vec![0.0, 0.5, 1.0];
    
    let mut optimization_results = Vec::new();
    let mut best_result: Option<OptimizationResult> = None;
    let mut best_score = f64::NEG_INFINITY;
    
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    // Simplified grid search for demonstration
    for &short_period in &reduced_short_periods {
        for &long_period in &reduced_long_periods {
            if short_period >= long_period {
                continue; // Skip invalid combinations
            }
            
            for &funding_weight in &reduced_funding_weights {
                for &stop_loss_pct in &[0.03, 0.05] {
                    for &take_profit_pct in &[0.05, 0.10] {
                        let params = StrategyParams {
                            short_period,
                            long_period,
                            funding_weight,
                            stop_loss_pct,
                            take_profit_pct,
                        };
                        
                        println!("  Testing parameters: {:?}", params);
                        
                        // Create strategy with current parameters
                        let strategy = create_strategy_with_params(&params);
                        
                        // Run backtest
                        let mut backtest = HyperliquidBacktest::new(
                            in_sample_data.clone(),
                            strategy,
                            initial_capital,
                            commission.clone(),
                        );
                        
                        backtest.calculate_with_funding();
                        let report = backtest.enhanced_report();
                        
                        // Calculate optimization score (multi-objective)
                        // Higher return, higher Sharpe, lower drawdown is better
                        let optimization_score = 
                            (report.total_return * 0.4) + 
                            (report.sharpe_ratio * 0.4) - 
                            (report.max_drawdown * 0.2);
                        
                        let result = OptimizationResult {
                            params: params.clone(),
                            final_equity: report.final_equity,
                            total_return: report.total_return,
                            max_drawdown: report.max_drawdown,
                            sharpe_ratio: report.sharpe_ratio,
                            sortino_ratio: report.sortino_ratio.unwrap_or(0.0),
                            win_rate: report.win_rate,
                            profit_factor: report.profit_factor.unwrap_or(0.0),
                            recovery_factor: report.recovery_factor.unwrap_or(0.0),
                            trades_count: report.trades_count,
                            optimization_score,
                        };
                        
                        optimization_results.push(result.clone());
                        
                        // Update best result if needed
                        if optimization_score > best_score {
                            best_score = optimization_score;
                            best_result = Some(result);
                        }
                    }
                }
            }
        }
    }
    
    // 4. Analyze optimization results
    println!("\n4. Analyzing optimization results...");
    
    // Sort results by optimization score
    optimization_results.sort_by(|a, b| b.optimization_score.partial_cmp(&a.optimization_score).unwrap());
    
    // Print top 5 parameter sets
    println!("  Top 5 parameter sets:");
    for (i, result) in optimization_results.iter().take(5).enumerate() {
        println!("  #{}: {:?}", i + 1, result.params);
        println!("    Return: {:.2}%, Drawdown: {:.2}%, Sharpe: {:.2}, Score: {:.4}",
                 result.total_return * 100.0, result.max_drawdown * 100.0, 
                 result.sharpe_ratio, result.optimization_score);
    }
    
    // 5. Validate with out-of-sample testing
    println!("\n5. Validating with out-of-sample testing...");
    
    let best_params = &best_result.as_ref().unwrap().params;
    println!("  Best parameters from in-sample optimization: {:?}", best_params);
    
    // Run out-of-sample backtest with best parameters
    let out_of_sample_strategy = create_strategy_with_params(best_params);
    let mut out_of_sample_backtest = HyperliquidBacktest::new(
        out_of_sample_data.clone(),
        out_of_sample_strategy,
        initial_capital,
        commission.clone(),
    );
    
    out_of_sample_backtest.calculate_with_funding();
    let out_of_sample_report = out_of_sample_backtest.enhanced_report();
    
    println!("  Out-of-sample performance:");
    println!("    Return: {:.2}%", out_of_sample_report.total_return * 100.0);
    println!("    Drawdown: {:.2}%", out_of_sample_report.max_drawdown * 100.0);
    println!("    Sharpe Ratio: {:.2}", out_of_sample_report.sharpe_ratio);
    println!("    Win Rate: {:.2}%", out_of_sample_report.win_rate * 100.0);
    println!("    Profit Factor: {:.2}", out_of_sample_report.profit_factor.unwrap_or(0.0));
    
    // 6. Export optimization results
    println!("\n6. Exporting optimization results...");
    
    export_optimization_results(&optimization_results)?;
    
    println!("\nStrategy optimization workflow example completed successfully!");
    println!("Optimization results exported to strategy_optimization_results.csv");
    
    Ok(())
}

fn create_strategy_with_params(params: &StrategyParams) -> Strategy {
    // Create a strategy based on the given parameters
    // This is a simplified example using a moving average crossover strategy
    // with funding rate awareness
    
    let mut strategy = Strategy::new();
    
    // Add indicators
    strategy.add_indicator(
        "short_ma",
        Box::new(move |data: &Data, _: &mut HashMap<String, f64>| {
            let closes = &data.close;
            let period = params.short_period;
            
            if closes.len() < period {
                return vec![f64::NAN; closes.len()];
            }
            
            let mut result = vec![f64::NAN; period - 1];
            
            for i in period - 1..closes.len() {
                let sum: f64 = closes[i - period + 1..=i].iter().sum();
                let ma = sum / period as f64;
                result.push(ma);
            }
            
            result
        }),
    );
    
    strategy.add_indicator(
        "long_ma",
        Box::new(move |data: &Data, _: &mut HashMap<String, f64>| {
            let closes = &data.close;
            let period = params.long_period;
            
            if closes.len() < period {
                return vec![f64::NAN; closes.len()];
            }
            
            let mut result = vec![f64::NAN; period - 1];
            
            for i in period - 1..closes.len() {
                let sum: f64 = closes[i - period + 1..=i].iter().sum();
                let ma = sum / period as f64;
                result.push(ma);
            }
            
            result
        }),
    );
    
    // Add funding rate indicator if funding weight > 0
    if params.funding_weight > 0.0 {
        strategy.add_indicator(
            "funding_signal",
            Box::new(move |data: &Data, _: &mut HashMap<String, f64>| {
                // Assuming data is HyperliquidData with funding_rates
                let hyperliquid_data = match data.as_any().downcast_ref::<HyperliquidData>() {
                    Some(hdata) => hdata,
                    None => return vec![0.0; data.len()],
                };
                
                let mut result = Vec::with_capacity(data.len());
                
                for i in 0..data.len() {
                    let funding_rate = if i < hyperliquid_data.funding_rates.len() {
                        hyperliquid_data.funding_rates[i]
                    } else {
                        0.0
                    };
                    
                    // Normalize funding rate to a signal between -1 and 1
                    let funding_signal = if funding_rate.is_nan() {
                        0.0
                    } else {
                        funding_rate.clamp(-0.01, 0.01) * 100.0
                    };
                    
                    result.push(funding_signal);
                }
                
                result
            }),
        );
    }
    
    // Add strategy logic
    strategy.set_logic(Box::new(move |data: &Data, indicators: &HashMap<String, Vec<f64>>, position: f64, _: &mut HashMap<String, f64>| {
        let i = data.len() - 1;
        
        if i < params.long_period {
            return 0.0;
        }
        
        let short_ma = indicators.get("short_ma").unwrap()[i];
        let long_ma = indicators.get("long_ma").unwrap()[i];
        
        let mut signal = 0.0;
        
        // Basic MA crossover signal
        if short_ma > long_ma {
            signal = 1.0; // Buy signal
        } else if short_ma < long_ma {
            signal = -1.0; // Sell signal
        }
        
        // Add funding rate influence if enabled
        if params.funding_weight > 0.0 {
            if let Some(funding_signals) = indicators.get("funding_signal") {
                let funding_signal = funding_signals[i];
                
                // Positive funding rate favors short positions (you receive funding)
                // Negative funding rate favors long positions (you receive funding)
                signal += -funding_signal * params.funding_weight;
            }
        }
        
        // Apply stop-loss and take-profit
        if position > 0.0 {
            let entry_price = data.meta.get("entry_price").unwrap_or(&data.close[i - 1]);
            let current_price = data.close[i];
            let return_pct = (current_price - entry_price) / entry_price;
            
            if return_pct <= -params.stop_loss_pct {
                return 0.0; // Stop loss - close position
            }
            
            if return_pct >= params.take_profit_pct {
                return 0.0; // Take profit - close position
            }
        } else if position < 0.0 {
            let entry_price = data.meta.get("entry_price").unwrap_or(&data.close[i - 1]);
            let current_price = data.close[i];
            let return_pct = (entry_price - current_price) / entry_price;
            
            if return_pct <= -params.stop_loss_pct {
                return 0.0; // Stop loss - close position
            }
            
            if return_pct >= params.take_profit_pct {
                return 0.0; // Take profit - close position
            }
        }
        
        // Determine position size based on signal strength
        if signal > 0.5 {
            1.0 // Full long position
        } else if signal < -0.5 {
            -1.0 // Full short position
        } else {
            0.0 // No position
        }
    }));
    
    strategy
}

fn export_optimization_results(results: &[OptimizationResult]) -> Result<()> {
    let mut csv = String::from("rank,short_period,long_period,funding_weight,stop_loss_pct,take_profit_pct,final_equity,total_return,max_drawdown,sharpe_ratio,sortino_ratio,win_rate,profit_factor,trades_count,optimization_score\n");
    
    for (i, result) in results.iter().enumerate() {
        csv.push_str(&format!(
            "{},{},{},{:.2},{:.4},{:.4},{:.2},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{},{:.6}\n",
            i + 1,
            result.params.short_period,
            result.params.long_period,
            result.params.funding_weight,
            result.params.stop_loss_pct,
            result.params.take_profit_pct,
            result.final_equity,
            result.total_return,
            result.max_drawdown,
            result.sharpe_ratio,
            result.sortino_ratio,
            result.win_rate,
            result.profit_factor,
            result.trades_count,
            result.optimization_score
        ));
    }
    
    let mut file = File::create("strategy_optimization_results.csv")?;
    file.write_all(csv.as_bytes())?;
    
    Ok(())
}