//! Indicators and analysis tools for funding rates and market data

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

/// Direction of funding rate
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FundingDirection {
    /// Positive funding rate (longs pay shorts)
    Positive,
    /// Negative funding rate (shorts pay longs)
    Negative,
    /// Neutral funding rate (close to zero)
    Neutral,
}

impl FundingDirection {
    /// Determine direction from funding rate value
    pub fn from_rate(rate: f64) -> Self {
        if rate > 0.0 {
            FundingDirection::Positive
        } else if rate < 0.0 {
            FundingDirection::Negative
        } else {
            FundingDirection::Neutral
        }
    }
}

/// Funding rate volatility analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingVolatility {
    /// Standard deviation of funding rates
    pub std_dev: f64,
    /// Coefficient of variation
    pub cv: f64,
    /// Is volatility high compared to historical average
    pub is_high: bool,
    /// Percentile of current volatility in historical distribution
    pub percentile: f64,
}

/// Calculate funding rate volatility
pub fn calculate_funding_volatility(rates: &[f64]) -> f64 {
    if rates.len() <= 1 {
        return 0.0;
    }
    
    let mean = rates.iter().sum::<f64>() / rates.len() as f64;
    let variance = rates.iter()
        .map(|&r| (r - mean).powi(2))
        .sum::<f64>() / (rates.len() - 1) as f64;
    
    variance.sqrt()
}

/// Funding rate momentum analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingMomentum {
    /// Direction of momentum
    pub direction: FundingDirection,
    /// Strength of momentum (0.0 to 1.0)
    pub strength: f64,
    /// Rate of change
    pub rate_of_change: f64,
    /// Is momentum accelerating
    pub is_accelerating: bool,
}

/// Calculate funding rate momentum
pub fn calculate_funding_momentum(rates: &[f64]) -> f64 {
    if rates.len() <= 1 {
        return 0.0;
    }
    
    // Simple linear regression slope
    let n = rates.len() as f64;
    let indices: Vec<f64> = (0..rates.len()).map(|i| i as f64).collect();
    
    let sum_x: f64 = indices.iter().sum();
    let sum_y: f64 = rates.iter().sum();
    let sum_xy: f64 = indices.iter().zip(rates.iter()).map(|(&x, &y)| x * y).sum();
    let sum_xx: f64 = indices.iter().map(|&x| x * x).sum();
    
    let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
    
    slope
}

/// Funding cycle analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingCycle {
    /// Period of the cycle in hours
    pub period_hours: usize,
    /// Strength of the cycle pattern (0.0 to 1.0)
    pub strength: f64,
    /// Is the cycle statistically significant
    pub is_significant: bool,
    /// Phase of the cycle (0.0 to 1.0)
    pub current_phase: f64,
}

/// Funding rate anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingAnomaly {
    /// Is the current rate anomalous
    pub is_anomaly: bool,
    /// How many standard deviations from the mean
    pub deviation: f64,
    /// Direction of the anomaly
    pub direction: FundingDirection,
    /// Potential cause of the anomaly
    pub potential_cause: String,
}

/// Funding arbitrage opportunity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingArbitrageOpportunity {
    /// Is there an arbitrage opportunity
    pub is_arbitrage: bool,
    /// Direction of the opportunity
    pub direction: FundingDirection,
    /// Annualized yield of the opportunity
    pub annualized_yield: f64,
    /// Payment per contract
    pub payment_per_contract: f64,
}

/// Calculate funding arbitrage opportunity
pub fn calculate_funding_arbitrage(funding_rate: f64, price: f64) -> FundingArbitrageOpportunity {
    let direction = FundingDirection::from_rate(funding_rate);
    let is_arbitrage = funding_rate.abs() > 0.0001; // Threshold for arbitrage
    let annualized_yield = funding_rate.abs() * 3.0 * 365.25; // 3 funding periods per day
    let payment_per_contract = funding_rate.abs() * price;
    
    FundingArbitrageOpportunity {
        is_arbitrage,
        direction,
        annualized_yield,
        payment_per_contract,
    }
}

/// Correlation between funding rate and price
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingPriceCorrelation {
    /// Correlation coefficient (-1.0 to 1.0)
    pub coefficient: f64,
    /// Is the correlation statistically significant
    pub is_significant: bool,
    /// Relationship description
    pub relationship: String,
    /// Time lag with strongest correlation
    pub optimal_lag: i32,
}

impl FundingPriceCorrelation {
    /// Calculate correlation between funding rates and prices
    pub fn calculate(funding_rates: &[f64], prices: &[f64]) -> Self {
        if funding_rates.len() != prices.len() || funding_rates.is_empty() {
            return Self {
                coefficient: 0.0,
                is_significant: false,
                relationship: "Unknown".to_string(),
                optimal_lag: 0,
            };
        }
        
        let n = funding_rates.len() as f64;
        let sum_x: f64 = funding_rates.iter().sum();
        let sum_y: f64 = prices.iter().sum();
        let sum_xy: f64 = funding_rates.iter().zip(prices.iter()).map(|(&x, &y)| x * y).sum();
        let sum_xx: f64 = funding_rates.iter().map(|&x| x * x).sum();
        let sum_yy: f64 = prices.iter().map(|&y| y * y).sum();
        
        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator = ((n * sum_xx - sum_x * sum_x) * (n * sum_yy - sum_y * sum_y)).sqrt();
        
        let coefficient = if denominator != 0.0 {
            numerator / denominator
        } else {
            0.0
        };
        
        let is_significant = coefficient.abs() > 0.5;
        let relationship = if coefficient > 0.7 {
            "Positive".to_string()
        } else if coefficient < -0.7 {
            "Negative".to_string()
        } else {
            "Weak".to_string()
        };
        
        Self {
            coefficient,
            is_significant,
            relationship,
            optimal_lag: 0, // Would require more complex analysis
        }
    }
}

/// Open interest data and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInterestData {
    /// Current open interest
    pub current_oi: f64,
    /// Change in open interest
    pub change: OpenInterestChange,
    /// Long/short ratio
    pub long_short_ratio: f64,
    /// Is open interest at historical high
    pub is_at_high: bool,
}

/// Open interest change analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInterestChange {
    /// Absolute change in open interest
    pub absolute_change: f64,
    /// Percentage change in open interest
    pub percentage_change: f64,
    /// USD value of the change
    pub usd_value_change: f64,
    /// Is open interest increasing
    pub is_increasing: bool,
    /// Is open interest decreasing
    pub is_decreasing: bool,
}

impl OpenInterestChange {
    /// Create a new OpenInterestChange
    pub fn new(prev_oi: f64, curr_oi: f64, price: f64) -> Self {
        let absolute_change = curr_oi - prev_oi;
        let percentage_change = if prev_oi > 0.0 {
            absolute_change / prev_oi
        } else {
            0.0
        };
        let usd_value_change = absolute_change * price;
        
        Self {
            absolute_change,
            percentage_change,
            usd_value_change,
            is_increasing: absolute_change > 0.0,
            is_decreasing: absolute_change < 0.0,
        }
    }
}

/// Liquidation data and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationData {
    /// Total liquidation amount
    pub total_amount: f64,
    /// Long liquidations
    pub long_liquidations: f64,
    /// Short liquidations
    pub short_liquidations: f64,
    /// Impact on market
    pub impact: LiquidationImpact,
}

/// Liquidation market impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationImpact {
    /// Liquidation as percentage of open interest
    pub liquidation_percentage: f64,
    /// USD value of liquidations
    pub usd_value: f64,
    /// Price impact percentage
    pub price_impact: f64,
    /// Is the liquidation significant
    pub is_significant: bool,
}

impl LiquidationImpact {
    /// Create a new LiquidationImpact
    pub fn new(liquidation_amount: f64, open_interest: f64, price: f64, price_impact: f64) -> Self {
        let liquidation_percentage = if open_interest > 0.0 {
            liquidation_amount / open_interest
        } else {
            0.0
        };
        let usd_value = liquidation_amount * price;
        let is_significant = liquidation_percentage > 0.05 || price_impact.abs() > 0.01;
        
        Self {
            liquidation_percentage,
            usd_value,
            price_impact,
            is_significant,
        }
    }
}

/// Basis indicator for futures vs spot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasisIndicator {
    /// Basis percentage
    pub basis: f64,
    /// Annualized basis
    pub annualized_basis: f64,
    /// Absolute basis amount
    pub basis_amount: f64,
    /// Is the basis widening
    pub is_widening: bool,
}

/// Calculate basis indicator
pub fn calculate_basis_indicator(spot_price: f64, futures_price: f64, days_to_expiry: f64) -> BasisIndicator {
    let basis_amount = futures_price - spot_price;
    let basis = basis_amount / spot_price;
    let annualized_basis = basis * (365.0 / days_to_expiry);
    
    BasisIndicator {
        basis,
        annualized_basis,
        basis_amount,
        is_widening: false, // Would need historical data to determine
    }
}

/// Funding rate prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingPrediction {
    /// Expected funding rate
    pub expected_rate: f64,
    /// Direction of the prediction
    pub direction: FundingDirection,
    /// Confidence in the prediction (0.0 to 1.0)
    pub confidence: f64,
    /// Time horizon of the prediction in hours
    pub horizon_hours: u32,
}

/// Configuration for funding rate prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingPredictionConfig {
    /// Number of periods to look back for analysis
    pub lookback_periods: usize,
    /// Weight for volatility in prediction
    pub volatility_weight: f64,
    /// Weight for momentum in prediction
    pub momentum_weight: f64,
    /// Weight for basis in prediction
    pub basis_weight: f64,
    /// Weight for correlation in prediction
    pub correlation_weight: f64,
}

impl Default for FundingPredictionConfig {
    fn default() -> Self {
        Self {
            lookback_periods: 48,
            volatility_weight: 0.2,
            momentum_weight: 0.3,
            basis_weight: 0.3,
            correlation_weight: 0.2,
        }
    }
}

/// Funding rate prediction model
pub trait FundingPredictionModel {
    /// Add a new funding rate observation
    fn add_observation(&mut self, rate: f64);
    
    /// Predict the next funding rate
    fn predict(&self) -> FundingPrediction;
    
    /// Get the volatility of funding rates
    fn get_volatility(&self) -> f64;
    
    /// Get the momentum of funding rates
    fn get_momentum(&self) -> f64;
    
    /// Detect funding cycle
    fn detect_funding_cycle(&self) -> FundingCycle;
    
    /// Detect funding anomaly
    fn detect_anomaly(&self) -> FundingAnomaly;
    
    /// Calculate correlation with another predictor
    fn correlation_with(&self, other: &dyn FundingPredictionModel) -> f64;
}

/// Funding rate predictor implementation
pub struct FundingRatePredictor {
    /// Configuration for the predictor
    config: FundingPredictionConfig,
    /// Historical funding rates
    rates: VecDeque<f64>,
}

impl FundingRatePredictor {
    /// Create a new FundingRatePredictor
    pub fn new(config: FundingPredictionConfig) -> Self {
        let capacity = config.lookback_periods;
        Self {
            config,
            rates: VecDeque::with_capacity(capacity),
        }
    }
}

impl FundingPredictionModel for FundingRatePredictor {
    fn add_observation(&mut self, rate: f64) {
        if self.rates.len() >= self.config.lookback_periods {
            self.rates.pop_front();
        }
        self.rates.push_back(rate);
    }
    
    fn predict(&self) -> FundingPrediction {
        if self.rates.is_empty() {
            return FundingPrediction {
                expected_rate: 0.0,
                direction: FundingDirection::Neutral,
                confidence: 0.0,
                horizon_hours: 8,
            };
        }
        
        // Simple prediction based on recent trend
        let rates: Vec<f64> = self.rates.iter().copied().collect();
        let momentum = calculate_funding_momentum(&rates);
        let volatility = calculate_funding_volatility(&rates);
        
        // Last observed rate
        let last_rate = *self.rates.back().unwrap();
        
        // Predict next rate based on momentum
        let expected_rate = last_rate + momentum;
        let direction = FundingDirection::from_rate(expected_rate);
        
        // Higher confidence with lower volatility and stronger momentum
        let confidence = 0.5 + 0.3 * (momentum.abs() / (volatility + 0.0001)).min(1.0);
        
        FundingPrediction {
            expected_rate,
            direction,
            confidence,
            horizon_hours: 8,
        }
    }
    
    fn get_volatility(&self) -> f64 {
        let rates: Vec<f64> = self.rates.iter().copied().collect();
        calculate_funding_volatility(&rates)
    }
    
    fn get_momentum(&self) -> f64 {
        let rates: Vec<f64> = self.rates.iter().copied().collect();
        calculate_funding_momentum(&rates)
    }
    
    fn detect_funding_cycle(&self) -> FundingCycle {
        // Simple cycle detection (in a real implementation, would use FFT or autocorrelation)
        FundingCycle {
            period_hours: 8, // Typical funding period
            strength: 0.7,
            is_significant: true,
            current_phase: 0.5,
        }
    }
    
    fn detect_anomaly(&self) -> FundingAnomaly {
        if self.rates.len() < 2 {
            return FundingAnomaly {
                is_anomaly: false,
                deviation: 0.0,
                direction: FundingDirection::Neutral,
                potential_cause: "Insufficient data".to_string(),
            };
        }
        
        let rates: Vec<f64> = self.rates.iter().copied().collect();
        let mean = rates.iter().sum::<f64>() / rates.len() as f64;
        let std_dev = calculate_funding_volatility(&rates);
        
        let last_rate = *self.rates.back().unwrap();
        let deviation = if std_dev > 0.0 {
            (last_rate - mean) / std_dev
        } else {
            0.0
        };
        
        let is_anomaly = deviation.abs() > 3.0; // 3 sigma rule
        let direction = FundingDirection::from_rate(last_rate);
        
        FundingAnomaly {
            is_anomaly,
            deviation: deviation.abs(),
            direction,
            potential_cause: if is_anomaly {
                "Significant market event".to_string()
            } else {
                "Normal market conditions".to_string()
            },
        }
    }
    
    fn correlation_with(&self, _other: &dyn FundingPredictionModel) -> f64 {
        // Simplified implementation - return a default correlation
        0.5
    }
}