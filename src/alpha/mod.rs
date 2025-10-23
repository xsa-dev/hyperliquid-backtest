//! Alpha research and evaluation pipeline.
//!
//! The module consumes feature series produced by [`crate::features`] and
//! evaluates their predictive power against a forward return target. It offers
//! pluggable alpha models, a convenience pipeline, and summary statistics that
//! can be rendered into reports or converted into trading signals.

use std::fmt;

use crate::data::HyperliquidData;
use crate::features::{FeatureSeries, FeatureSet};
use crate::report::AlphaReport;

/// Target time series used for alpha evaluation.
#[derive(Debug, Clone)]
pub struct AlphaTarget {
    horizon: usize,
    values: Vec<f64>,
}

impl AlphaTarget {
    /// Build a forward return target from close prices.
    pub fn forward_returns(data: &HyperliquidData, horizon: usize) -> Self {
        let horizon = horizon.max(1);
        let mut values = Vec::new();
        if data.close.len() > horizon {
            for idx in 0..data.close.len() - horizon {
                let future = data.close[idx + horizon];
                let current = data.close[idx];
                if current != 0.0 {
                    values.push((future / current) - 1.0);
                } else {
                    values.push(f64::NAN);
                }
            }
        }
        Self { horizon, values }
    }

    /// Borrow the raw target values.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// Forward horizon used to build the target.
    pub fn horizon(&self) -> usize {
        self.horizon
    }

    /// Number of samples available in the target.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the target contains no samples.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Statistics returned by alpha evaluation.
#[derive(Debug, Clone)]
pub struct AlphaEvaluation {
    /// Name of the feature that was evaluated.
    pub feature_name: String,
    /// Name of the model used for the evaluation.
    pub model_name: String,
    /// Pearson correlation between feature and forward return.
    pub ic: f64,
    /// Sharpe ratio of the sign-based signal derived from the feature.
    pub sharpe: f64,
    /// Average sign-based return.
    pub mean_return: f64,
    /// Normalised feature values used to build signals.
    pub scores: Vec<f64>,
    /// Per-sample product of normalised feature and target (rolling IC proxy).
    pub ic_series: Vec<f64>,
    /// Number of observations used during the evaluation.
    pub sample_size: usize,
}

impl AlphaEvaluation {
    /// Whether the evaluation produced at least one score.
    pub fn has_observations(&self) -> bool {
        self.sample_size > 0
    }
}

/// Collection of evaluations produced by a pipeline run.
#[derive(Debug, Clone)]
pub struct AlphaEvaluationSet {
    evaluations: Vec<AlphaEvaluation>,
}

impl AlphaEvaluationSet {
    /// Create a new set from raw evaluations.
    pub fn new(evaluations: Vec<AlphaEvaluation>) -> Self {
        Self { evaluations }
    }

    /// Borrow the underlying evaluations.
    pub fn iter(&self) -> impl Iterator<Item = &AlphaEvaluation> {
        self.evaluations.iter()
    }

    /// Number of evaluations.
    pub fn len(&self) -> usize {
        self.evaluations.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.evaluations.is_empty()
    }

    /// Filter evaluations by absolute IC threshold.
    pub fn filter_by_ic(self, threshold: f64) -> Self {
        let evaluations = self
            .evaluations
            .into_iter()
            .filter(|eval| eval.ic.abs() >= threshold)
            .collect();
        Self { evaluations }
    }

    /// Filter evaluations by minimum Sharpe ratio.
    pub fn filter_by_sharpe(self, threshold: f64) -> Self {
        let evaluations = self
            .evaluations
            .into_iter()
            .filter(|eval| eval.sharpe.is_finite() && eval.sharpe >= threshold)
            .collect();
        Self { evaluations }
    }

    /// Convert the evaluation set into a report.
    pub fn to_report(&self) -> AlphaReport {
        AlphaReport::from_evaluations(self.evaluations.clone())
    }

    /// Consume the set and return the inner vector.
    pub fn into_vec(self) -> Vec<AlphaEvaluation> {
        self.evaluations
    }
}

/// Trait implemented by alpha scoring models.
pub trait AlphaModel: fmt::Debug + Send + Sync {
    /// Name of the model.
    fn name(&self) -> &'static str;

    /// Evaluate a single feature against the target.
    fn evaluate(&self, feature: &FeatureSeries, target: &AlphaTarget) -> Option<AlphaEvaluation>;
}

/// Simple correlation-based alpha model.
#[derive(Debug, Default, Clone, Copy)]
pub struct CorrelationAlpha;

impl CorrelationAlpha {
    fn build_evaluation(
        &self,
        feature: &FeatureSeries,
        samples: Vec<(f64, f64)>,
    ) -> AlphaEvaluation {
        let (feature_values, target_values): (Vec<_>, Vec<_>) = samples.into_iter().unzip();
        let sample_size = feature_values.len();

        let feature_mean = mean(&feature_values);
        let target_mean = mean(&target_values);
        let feature_std = std_dev(&feature_values, feature_mean);
        let target_std = std_dev(&target_values, target_mean);

        let ic = if feature_std > 0.0 && target_std > 0.0 {
            covariance(&feature_values, feature_mean, &target_values, target_mean)
                / (feature_std * target_std)
        } else {
            0.0
        };

        let scores = z_scores(&feature_values, feature_mean, feature_std);
        let normalised_target = z_scores(&target_values, target_mean, target_std);
        let ic_series = scores
            .iter()
            .zip(normalised_target.iter())
            .map(|(f, t)| f * t)
            .collect::<Vec<_>>();

        let mut signal_returns = Vec::with_capacity(scores.len());
        for (score, target) in scores.iter().zip(target_values.iter()) {
            let sign = if *score > 0.0 {
                1.0
            } else if *score < 0.0 {
                -1.0
            } else {
                0.0
            };
            signal_returns.push(sign * target);
        }

        let mean_return = if signal_returns.is_empty() {
            0.0
        } else {
            mean(&signal_returns)
        };
        let signal_std = std_dev(&signal_returns, mean_return);
        let sharpe = if signal_std > 0.0 {
            mean_return / signal_std
        } else {
            0.0
        };

        AlphaEvaluation {
            feature_name: feature.name().to_string(),
            model_name: self.name().to_string(),
            ic,
            sharpe,
            mean_return,
            scores,
            ic_series,
            sample_size,
        }
    }
}

impl AlphaModel for CorrelationAlpha {
    fn name(&self) -> &'static str {
        "correlation"
    }

    fn evaluate(&self, feature: &FeatureSeries, target: &AlphaTarget) -> Option<AlphaEvaluation> {
        if target.is_empty() || feature.is_empty() {
            return None;
        }

        let target_len = target.len();
        let values = feature.values();
        let sample_len = values.len().min(target_len);
        if sample_len < 2 {
            return None;
        }

        let mut samples = Vec::with_capacity(sample_len);
        for idx in 0..sample_len {
            let feature_value = values[idx];
            let target_value = target.values()[idx];
            if feature_value.is_finite() && target_value.is_finite() {
                samples.push((feature_value, target_value));
            }
        }

        if samples.len() < 2 {
            return None;
        }

        Some(self.build_evaluation(feature, samples))
    }
}

/// Alpha pipeline orchestrating feature evaluation.
pub struct AlphaPipeline<'a> {
    data: &'a HyperliquidData,
    features: FeatureSet,
    target: AlphaTarget,
}

impl<'a> AlphaPipeline<'a> {
    /// Create a new pipeline.
    pub fn new(data: &'a HyperliquidData, features: FeatureSet, horizon: usize) -> Self {
        let target = AlphaTarget::forward_returns(data, horizon);
        Self {
            data,
            features,
            target,
        }
    }

    /// Borrow the market data powering the pipeline.
    pub fn data(&self) -> &'a HyperliquidData {
        self.data
    }

    /// Borrow the computed feature set.
    pub fn features(&self) -> &FeatureSet {
        &self.features
    }

    /// Borrow the evaluation target.
    pub fn target(&self) -> &AlphaTarget {
        &self.target
    }

    /// Evaluate all features with the supplied alpha model.
    pub fn evaluate_all<M>(&self, model: &M) -> AlphaEvaluationSet
    where
        M: AlphaModel,
    {
        let mut evaluations = Vec::new();
        for feature in self.features.iter() {
            if let Some(result) = model.evaluate(feature, &self.target) {
                evaluations.push(result);
            }
        }
        AlphaEvaluationSet::new(evaluations)
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn std_dev(values: &[f64], mean: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let variance = values
        .iter()
        .map(|value| {
            let diff = value - mean;
            diff * diff
        })
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn covariance(feature: &[f64], feature_mean: f64, target: &[f64], target_mean: f64) -> f64 {
    feature
        .iter()
        .zip(target.iter())
        .map(|(f, t)| (f - feature_mean) * (t - target_mean))
        .sum::<f64>()
        / feature.len() as f64
}

fn z_scores(values: &[f64], mean: f64, std_dev: f64) -> Vec<f64> {
    if std_dev == 0.0 {
        return vec![0.0; values.len()];
    }
    values
        .iter()
        .map(|value| (value - mean) / std_dev)
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{
        compute_feature_set, Feature, LagReturnFeature, RsiFeature, VolatilityFeature,
    };
    use chrono::{FixedOffset, TimeZone};

    fn boxed_feature<F>(feature: F) -> Box<dyn Feature + Send + Sync>
    where
        F: Feature + Send + Sync + 'static,
    {
        Box::new(feature)
    }

    fn mock_data() -> HyperliquidData {
        let tz = FixedOffset::east_opt(0).unwrap();
        let mut datetime = Vec::new();
        let mut open = Vec::new();
        let mut high = Vec::new();
        let mut low = Vec::new();
        let mut close = Vec::new();
        let mut volume = Vec::new();
        let mut funding_rates = Vec::new();
        for i in 0..40 {
            let base = 100.0 + i as f64 * 0.5;
            datetime.push(tz.timestamp_opt(i as i64, 0).unwrap());
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + (i % 3) as f64 * 0.2);
            volume.push(2_000.0 + i as f64 * 5.0);
            funding_rates.push(0.0001);
        }

        HyperliquidData {
            symbol: "BTC".to_string(),
            datetime,
            open,
            high,
            low,
            close,
            volume,
            funding_rates,
        }
    }

    #[test]
    fn correlation_alpha_returns_evaluations() {
        let data = mock_data();
        let feature_set = compute_feature_set(
            &data,
            vec![
                boxed_feature(RsiFeature { period: 5 }),
                boxed_feature(VolatilityFeature { window: 10 }),
                boxed_feature(LagReturnFeature { lag: 1 }),
            ],
        );
        let pipeline = AlphaPipeline::new(&data, feature_set, 1);
        let evaluations = pipeline.evaluate_all(&CorrelationAlpha);

        assert!(!evaluations.is_empty());
        for evaluation in evaluations.iter() {
            assert_eq!(evaluation.model_name, "correlation");
            assert!(evaluation.sample_size > 0);
        }
    }

    #[test]
    fn evaluation_filters_work() {
        let data = mock_data();
        let feature_set = compute_feature_set(
            &data,
            vec![
                boxed_feature(RsiFeature { period: 5 }),
                boxed_feature(VolatilityFeature { window: 5 }),
                boxed_feature(LagReturnFeature { lag: 2 }),
            ],
        );
        let pipeline = AlphaPipeline::new(&data, feature_set, 1);
        let evaluations = pipeline
            .evaluate_all(&CorrelationAlpha)
            .filter_by_ic(0.01)
            .filter_by_sharpe(-10.0);

        assert!(evaluations.len() <= 3);
    }
}
