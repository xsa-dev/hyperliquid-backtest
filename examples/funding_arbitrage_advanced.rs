use chrono::{Duration, Utc};
use hyperliquid_backtester::prelude::*;
use hyperliquid_backtester::indicators::*;
use rs_backtester::prelude::*;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;

/// # Advanced Funding Arbitrage Strategy Example
///
/// This example demonstrates a sophisticated funding arbitrage strategy that:
/// - Monitors funding rates across multiple timeframes
/// - Uses predictive models to forecast funding rate changes
/// - Implements dynamic position sizing based on funding rate volatility
/// - Manages risk through correlation analysis and position limits
/// - Tracks performance across different market conditions

#[tokio::main]
async fn main() -> Result<()> {
    println!("Advanced Funding Arbitrage Strategy Example");
    println!("==========================================\n");

    // Fetch historical data for multiple assets
    let end_time = Utc::now().timestamp() as u64;
    let start_time = end_time - (90 * 24 * 3600); // 90 days of data
    
    println!("Fetching multi-asset data for funding arbitrage analysis...");
    
    // Fetch data for BTC, ETH, and SOL
    let btc_data = HyperliquidData::fetch_btc("1h", start_time, end_time).await?;
    let eth_data = HyperliquidData::fetch_eth("1h", start_time, end_time).await?;
    let sol_data = HyperliquidData::fetch("SOL", "1h", start_time, end_time).await?;
    
    println!("Data fetched:");
    println!("  BTC: {} data points", btc_data.len());
    println!("  ETH: {} data points", eth_data.len());
    println!("  SOL: {} data points", sol_data.len());
    
    // Initialize funding rate predictors for each asset
    let prediction_config = FundingPredictionConfig {
        lookback_periods: 48, // 48 hours lookback
        volatility_weight: 0.25,
        momentum_weight: 0.35,
        basis_weight: 0.25,
        correlation_weight: 0.15,
    };
    
    let mut btc_predictor = FundingRatePredictor::new(prediction_config.clone());
    let mut eth_predictor = FundingRatePredictor::new(prediction_config.clone());
    let mut sol_predictor = FundingRatePredictor::new(prediction_config.clone());
    
    // Advanced funding arbitrage strategy
    let mut advanced_funding_strategy = Strategy::new();
    
    // Strategy state
    let mut position_tracker = HashMap::new();
    let mut funding_history = HashMap::new();
    let mut performance_metrics = Vec::new();
    
    // Strategy parameters
    let funding_threshold_high = 0.001; // 0.1% per 8h - strong signal
    let funding_threshold_low = 0.0005; // 0.05% per 8h - weak signal
    let max_position_per_asset = 0.3; // 30% of capital per asset
    let correlation_threshold = 0.7; // Reduce positions if correlation > 70%
    let volatility_adjustment = true; // Adjust position size based on volatility
    
    advanced_funding_strategy.next(Box::new(move |ctx, data| {
        let current_index = data.index;
        
        // Skip if not enough data for analysis
        if current_index < prediction_config.lookback_periods {
            return;
        }
        
        // Get current funding rates (simulated multi-asset access)
        let btc_funding = if current_index < btc_data.funding_rates.len() {
            btc_data.funding_rates[current_index]
        } else {
            return;
        };
        
        let eth_funding = if current_index < eth_data.funding_rates.len() {
            eth_data.funding_rates[current_index]
        } else {
            return;
        };
        
        let sol_funding = if current_index < sol_data.funding_rates.len() {
            sol_data.funding_rates[current_index]
        } else {
            return;
        };
        
        // Skip if any funding rate is invalid
        if btc_funding.is_nan() || eth_funding.is_nan() || sol_funding.is_nan() {
            return;
        }
        
        // Update predictors with new data
        btc_predictor.add_observation(btc_funding);
        eth_predictor.add_observation(eth_funding);
        sol_predictor.add_observation(sol_funding);
        
        // Get predictions
        let btc_prediction = btc_predictor.predict();
        let eth_prediction = eth_predictor.predict();
        let sol_prediction = sol_predictor.predict();
        
        // Calculate funding rate volatilities
        let btc_volatility = btc_predictor.get_volatility();
        let eth_volatility = eth_predictor.get_volatility();
        let sol_volatility = sol_predictor.get_volatility();
        
        // Calculate correlations between funding rates
        let btc_eth_correlation = btc_predictor.correlation_with(&eth_predictor);
        let btc_sol_correlation = btc_predictor.correlation_with(&sol_predictor);
        let eth_sol_correlation = eth_predictor.correlation_with(&sol_predictor);
        
        // Current positions
        let current_position = ctx.position();
        
        // Funding arbitrage logic
        let mut target_position = 0.0;
        let mut trade_reason = String::new();
        
        // Analyze BTC funding opportunity
        if btc_funding.abs() > funding_threshold_high && btc_prediction.confidence > 0.6 {
            let position_size = if volatility_adjustment {
                max_position_per_asset * (1.0 - btc_volatility.min(0.5))
            } else {
                max_position_per_asset
            };
            
            if btc_funding > 0.0 {
                // Positive funding - shorts pay longs, go long
                target_position += position_size;
                trade_reason.push_str("BTC+funding ");
            } else {
                // Negative funding - longs pay shorts, go short
                target_position -= position_size;
                trade_reason.push_str("BTC-funding ");
            }
        }
        
        // Analyze ETH funding opportunity
        if eth_funding.abs() > funding_threshold_low && eth_prediction.confidence > 0.5 {
            let position_size = if volatility_adjustment {
                max_position_per_asset * 0.7 * (1.0 - eth_volatility.min(0.5))
            } else {
                max_position_per_asset * 0.7
            };
            
            // Reduce position if highly correlated with BTC
            let correlation_adjustment = if btc_eth_correlation > correlation_threshold {
                0.5
            } else {
                1.0
            };
            
            if eth_funding > 0.0 {
                target_position += position_size * correlation_adjustment;
                trade_reason.push_str("ETH+funding ");
            } else {
                target_position -= position_size * correlation_adjustment;
                trade_reason.push_str("ETH-funding ");
            }
        }
        
        // Analyze SOL funding opportunity
        if sol_funding.abs() > funding_threshold_low && sol_prediction.confidence > 0.4 {
            let position_size = if volatility_adjustment {
                max_position_per_asset * 0.5 * (1.0 - sol_volatility.min(0.5))
            } else {
                max_position_per_asset * 0.5
            };
            
            // Reduce position if highly correlated with other assets
            let max_correlation = btc_sol_correlation.max(eth_sol_correlation);
            let correlation_adjustment = if max_correlation > correlation_threshold {
                0.3
            } else {
                1.0
            };
            
            if sol_funding > 0.0 {
                target_position += position_size * correlation_adjustment;
                trade_reason.push_str("SOL+funding ");
            } else {
                target_position -= position_size * correlation_adjustment;
                trade_reason.push_str("SOL-funding ");
            }
        }
        
        // Risk management: limit total position
        target_position = target_position.max(-1.0).min(1.0);
        
        // Execute trades if position change is significant
        let position_change = target_position - current_position;
        if position_change.abs() > 0.1 {
            ctx.entry_qty(target_position);
            
            // Log trade decision
            if !trade_reason.is_empty() {
                println!("Trade at index {}: {} -> {} ({})", 
                    current_index, current_position, target_position, trade_reason.trim());
            }
        }
        
        // Track performance metrics
        if current_index % 24 == 0 { // Every 24 hours
            let equity = ctx.equity();
            performance_metrics.push((current_index, equity, btc_funding, eth_funding, sol_funding));
        }
    }));
    
    // Run the advanced funding arbitrage backtest
    println!("\nRunning advanced funding arbitrage backtest...");
    
    let initial_capital = 50000.0; // $50,000
    let commission = HyperliquidCommission {
        maker_rate: 0.0002,  // 0.02% maker fee
        taker_rate: 0.0005,  // 0.05% taker fee
        funding_enabled: true,
    };
    
    let mut backtest = HyperliquidBacktest::new(
        btc_data.clone(), // Primary asset for backtest
        advanced_funding_strategy,
        initial_capital,
        commission,
    );
    
    backtest.calculate_with_funding();
    let stats = backtest.stats();
    let funding_report = backtest.funding_report();
    
    // Generate comprehensive performance report
    println!("\nAdvanced Funding Arbitrage Results:");
    println!("==================================");
    println!("Initial Capital: ${:.2}", initial_capital);
    println!("Final Equity: ${:.2}", stats.equity);
    println!("Net Profit: ${:.2} ({:.2}%)", stats.net_profit, (stats.net_profit / initial_capital) * 100.0);
    println!("Max Drawdown: {:.2}%", stats.max_drawdown * 100.0);
    println!("Sharpe Ratio: {:.3}", stats.sharpe_ratio);
    println!("Total Trades: {}", stats.total_trades);
    println!("Win Rate: {:.2}%", stats.win_rate * 100.0);
    
    println!("\nFunding-Specific Metrics:");
    println!("Total Funding Received: ${:.2}", funding_report.total_funding_received);
    println!("Total Funding Paid: ${:.2}", funding_report.total_funding_paid);
    println!("Net Funding PnL: ${:.2}", funding_report.net_funding_pnl);
    println!("Funding Contribution to Returns: {:.2}%", 
        (funding_report.net_funding_pnl / stats.net_profit) * 100.0);
    
    // Export detailed results
    println!("\nExporting detailed results...");
    
    // Export enhanced CSV with multi-asset funding data
    let enhanced_export = backtest.enhanced_csv_export()?;
    let csv_file = "advanced_funding_arbitrage.csv";
    let mut file = File::create(csv_file)?;
    file.write_all(enhanced_export.as_bytes())?;
    println!("Detailed results exported to {}", csv_file);
    
    // Export funding analysis
    let funding_analysis = format!(
        "timestamp,btc_funding,eth_funding,sol_funding,btc_prediction,eth_prediction,sol_prediction,portfolio_value\n{}",
        performance_metrics.iter()
            .map(|(idx, equity, btc_f, eth_f, sol_f)| {
                format!("{},{},{},{},{},{},{},{}", 
                    btc_data.datetime[*idx].timestamp(),
                    btc_f, eth_f, sol_f,
                    "N/A", "N/A", "N/A", // Predictions would need to be stored
                    equity)
            })
            .collect::<Vec<_>>()
            .join("\n")
    );
    
    let analysis_file = "funding_analysis.csv";
    let mut file = File::create(analysis_file)?;
    file.write_all(funding_analysis.as_bytes())?;
    println!("Funding analysis exported to {}", analysis_file);
    
    // Performance attribution analysis
    println!("\nPerformance Attribution:");
    println!("Trading PnL: ${:.2} ({:.1}%)", 
        stats.net_profit - funding_report.net_funding_pnl,
        ((stats.net_profit - funding_report.net_funding_pnl) / stats.net_profit) * 100.0);
    println!("Funding PnL: ${:.2} ({:.1}%)", 
        funding_report.net_funding_pnl,
        (funding_report.net_funding_pnl / stats.net_profit) * 100.0);
    
    println!("\nStrategy Insights:");
    println!("- Multi-asset approach provides diversification");
    println!("- Correlation analysis helps avoid over-concentration");
    println!("- Volatility-adjusted position sizing improves risk management");
    println!("- Predictive models enhance timing of entries/exits");
    
    println!("\nAdvanced funding arbitrage example completed successfully!");
    
    Ok(())
}