//! Funding report and analysis tools

use crate::errors::Result;
use crate::data::HyperliquidData;
use crate::backtest::FundingPayment;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, FixedOffset};

/// Funding rate point with timestamp and rate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRatePoint {
    /// Timestamp of the funding rate
    pub timestamp: DateTime<FixedOffset>,
    /// Funding rate value
    pub rate: f64,
}

/// Funding distribution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingDistribution {
    /// Mean funding rate
    pub mean: f64,
    /// Median funding rate
    pub median: f64,
    /// Standard deviation of funding rates
    pub std_dev: f64,
    /// Minimum funding rate
    pub min: f64,
    /// Maximum funding rate
    pub max: f64,
    /// Skewness of the distribution
    pub skewness: f64,
    /// Kurtosis of the distribution
    pub kurtosis: f64,
    /// Percentiles (10%, 25%, 75%, 90%)
    pub percentiles: [f64; 4],
}

/// Statistics about funding direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingDirectionStats {
    /// Number of positive funding periods
    pub positive_count: usize,
    /// Number of negative funding periods
    pub negative_count: usize,
    /// Number of zero funding periods
    pub zero_count: usize,
    /// Percentage of positive funding periods
    pub positive_percentage: f64,
    /// Percentage of negative funding periods
    pub negative_percentage: f64,
    /// Average positive funding rate
    pub avg_positive_rate: f64,
    /// Average negative funding rate
    pub avg_negative_rate: f64,
    /// Longest streak of positive funding
    pub longest_positive_streak: usize,
    /// Longest streak of negative funding
    pub longest_negative_streak: usize,
}

/// Funding metrics for a specific period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingPeriodMetric {
    /// Period name (e.g., "Daily", "Weekly", "Monthly")
    pub period_name: String,
    /// Average funding rate for the period
    pub avg_rate: f64,
    /// Total funding PnL for the period
    pub total_pnl: f64,
    /// Volatility of funding rates in the period
    pub volatility: f64,
    /// Sharpe ratio of funding returns
    pub sharpe: f64,
}

/// Funding metrics by different time periods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingMetricsByPeriod {
    /// Daily metrics
    pub daily: Vec<FundingPeriodMetric>,
    /// Weekly metrics
    pub weekly: Vec<FundingPeriodMetric>,
    /// Monthly metrics
    pub monthly: Vec<FundingPeriodMetric>,
}

/// Main funding report structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingReport {
    /// Symbol/ticker
    pub symbol: String,
    /// Total funding PnL
    pub net_funding_pnl: f64,
    /// Total funding received
    pub total_funding_received: f64,
    /// Total funding paid
    pub total_funding_paid: f64,
    /// Number of funding payments
    pub payment_count: usize,
    /// Average funding rate
    pub average_rate: f64,
    /// Funding rate volatility
    pub rate_volatility: f64,
    /// Funding rate distribution
    pub distribution: FundingDistribution,
    /// Funding direction statistics
    pub direction_stats: FundingDirectionStats,
    /// Funding payments history
    pub payments: Vec<FundingPayment>,
    /// Funding rates history
    pub rates: Vec<FundingRatePoint>,
    /// Metrics by period
    pub metrics_by_period: FundingMetricsByPeriod,
}

impl FundingReport {
    /// Create a new FundingReport
    pub fn new(
        symbol: &str,
        data: &HyperliquidData,
        position_sizes: &[f64],
        payments: Vec<FundingPayment>,
        net_funding_pnl: f64,
    ) -> Result<Self> {
        // Extract funding rates from data
        let mut rates = Vec::new();
        for (i, &timestamp) in data.datetime.iter().enumerate() {
            let rate = data.funding_rates[i];
            if !rate.is_nan() {
                rates.push(FundingRatePoint {
                    timestamp,
                    rate,
                });
            }
        }
        
        // Calculate statistics
        let rate_values: Vec<f64> = rates.iter().map(|r| r.rate).collect();
        let distribution = Self::calculate_funding_distribution(&rate_values)?;
        
        let direction_stats = Self::calculate_direction_stats(&rates);
        
        // Calculate total funding received and paid
        let mut total_received = 0.0;
        let mut total_paid = 0.0;
        
        for payment in &payments {
            if payment.payment_amount > 0.0 {
                total_received += payment.payment_amount;
            } else {
                total_paid += payment.payment_amount.abs();
            }
        }
        
        // Calculate average rate and volatility
        let average_rate = if !rates.is_empty() {
            rate_values.iter().sum::<f64>() / rate_values.len() as f64
        } else {
            0.0
        };
        
        let rate_volatility = if rate_values.len() > 1 {
            let mean = average_rate;
            let variance = rate_values.iter()
                .map(|&r| (r - mean).powi(2))
                .sum::<f64>() / (rate_values.len() - 1) as f64;
            variance.sqrt()
        } else {
            0.0
        };
        
        // Create metrics by period (simplified)
        let metrics_by_period = FundingMetricsByPeriod {
            daily: vec![FundingPeriodMetric {
                period_name: "Daily".to_string(),
                avg_rate: average_rate,
                total_pnl: net_funding_pnl / 30.0, // Simplified
                volatility: rate_volatility,
                sharpe: if rate_volatility > 0.0 { average_rate / rate_volatility } else { 0.0 },
            }],
            weekly: vec![FundingPeriodMetric {
                period_name: "Weekly".to_string(),
                avg_rate: average_rate,
                total_pnl: net_funding_pnl / 4.0, // Simplified
                volatility: rate_volatility,
                sharpe: if rate_volatility > 0.0 { average_rate / rate_volatility } else { 0.0 },
            }],
            monthly: vec![FundingPeriodMetric {
                period_name: "Monthly".to_string(),
                avg_rate: average_rate,
                total_pnl: net_funding_pnl,
                volatility: rate_volatility,
                sharpe: if rate_volatility > 0.0 { average_rate / rate_volatility } else { 0.0 },
            }],
        };
        
        Ok(Self {
            symbol: symbol.to_string(),
            net_funding_pnl,
            total_funding_received: total_received,
            total_funding_paid: total_paid,
            payment_count: payments.len(),
            average_rate,
            rate_volatility,
            distribution,
            direction_stats,
            payments,
            rates,
            metrics_by_period,
        })
    }
    
    /// Calculate funding distribution statistics
    fn calculate_funding_distribution(rates: &[f64]) -> Result<FundingDistribution> {
        if rates.is_empty() {
            return Ok(FundingDistribution {
                mean: 0.0,
                median: 0.0,
                std_dev: 0.0,
                min: 0.0,
                max: 0.0,
                skewness: 0.0,
                kurtosis: 0.0,
                percentiles: [0.0, 0.0, 0.0, 0.0],
            });
        }
        
        // Calculate basic statistics
        let n = rates.len();
        let mean = rates.iter().sum::<f64>() / n as f64;
        
        // Sort rates for percentiles and median
        let mut sorted_rates = rates.to_vec();
        sorted_rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let median = if n % 2 == 0 {
            (sorted_rates[n/2 - 1] + sorted_rates[n/2]) / 2.0
        } else {
            sorted_rates[n/2]
        };
        
        let min = sorted_rates[0];
        let max = sorted_rates[n - 1];
        
        // Calculate standard deviation
        let variance = rates.iter()
            .map(|&r| (r - mean).powi(2))
            .sum::<f64>() / (n - 1) as f64;
        let std_dev = variance.sqrt();
        
        // Calculate skewness and kurtosis
        let m3 = rates.iter()
            .map(|&r| (r - mean).powi(3))
            .sum::<f64>() / n as f64;
        let m4 = rates.iter()
            .map(|&r| (r - mean).powi(4))
            .sum::<f64>() / n as f64;
        
        let skewness = if std_dev > 0.0 { m3 / std_dev.powi(3) } else { 0.0 };
        let kurtosis = if std_dev > 0.0 { m4 / std_dev.powi(4) - 3.0 } else { 0.0 };
        
        // Calculate percentiles
        let p10_idx = (n as f64 * 0.1).round() as usize;
        let p25_idx = (n as f64 * 0.25).round() as usize;
        let p75_idx = (n as f64 * 0.75).round() as usize;
        let p90_idx = (n as f64 * 0.9).round() as usize;
        
        let percentiles = [
            sorted_rates[p10_idx.min(n - 1)],
            sorted_rates[p25_idx.min(n - 1)],
            sorted_rates[p75_idx.min(n - 1)],
            sorted_rates[p90_idx.min(n - 1)],
        ];
        
        Ok(FundingDistribution {
            mean,
            median,
            std_dev,
            min,
            max,
            skewness,
            kurtosis,
            percentiles,
        })
    }
    
    /// Calculate funding direction statistics
    fn calculate_direction_stats(rates: &[FundingRatePoint]) -> FundingDirectionStats {
        let mut positive_count = 0;
        let mut negative_count = 0;
        let mut zero_count = 0;
        
        let mut positive_sum = 0.0;
        let mut negative_sum = 0.0;
        
        let mut current_positive_streak = 0;
        let mut current_negative_streak = 0;
        let mut longest_positive_streak = 0;
        let mut longest_negative_streak = 0;
        
        for rate_point in rates {
            let rate = rate_point.rate;
            
            if rate > 0.0 {
                positive_count += 1;
                positive_sum += rate;
                current_positive_streak += 1;
                current_negative_streak = 0;
                longest_positive_streak = longest_positive_streak.max(current_positive_streak);
            } else if rate < 0.0 {
                negative_count += 1;
                negative_sum += rate;
                current_negative_streak += 1;
                current_positive_streak = 0;
                longest_negative_streak = longest_negative_streak.max(current_negative_streak);
            } else {
                zero_count += 1;
                current_positive_streak = 0;
                current_negative_streak = 0;
            }
        }
        
        let total_count = positive_count + negative_count + zero_count;
        let positive_percentage = if total_count > 0 {
            positive_count as f64 / total_count as f64
        } else {
            0.0
        };
        
        let negative_percentage = if total_count > 0 {
            negative_count as f64 / total_count as f64
        } else {
            0.0
        };
        
        let avg_positive_rate = if positive_count > 0 {
            positive_sum / positive_count as f64
        } else {
            0.0
        };
        
        let avg_negative_rate = if negative_count > 0 {
            negative_sum / negative_count as f64
        } else {
            0.0
        };
        
        FundingDirectionStats {
            positive_count,
            negative_count,
            zero_count,
            positive_percentage,
            negative_percentage,
            avg_positive_rate,
            avg_negative_rate,
            longest_positive_streak,
            longest_negative_streak,
        }
    }
}