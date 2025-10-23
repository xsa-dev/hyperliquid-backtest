//! Feature engineering primitives used by the alpha research pipeline.
//!
//! The module provides a small registry and a set of reusable feature
//! implementations that operate directly on [`HyperliquidData`](crate::data::HyperliquidData).
//! Each feature exposes a [`Feature`] trait implementation that converts
//! market data into a [`FeatureSeries`].
//!
//! The goal of the module is to keep the feature layer declarative and easy
//! to extend. Features can be collected either through the convenience
//! [`compute_feature_set`] function or by registering them with a
//! [`FeatureRegistry`]. The resulting [`FeatureSet`] is then consumed by the
//! alpha evaluation pipeline.

use crate::data::HyperliquidData;

/// Shared context passed to feature implementations.
pub struct FeatureContext<'a> {
    data: &'a HyperliquidData,
}

impl<'a> FeatureContext<'a> {
    /// Create a new feature context from market data.
    pub fn new(data: &'a HyperliquidData) -> Self {
        Self { data }
    }

    /// Borrow the underlying market data.
    pub fn data(&self) -> &'a HyperliquidData {
        self.data
    }
}

/// Output series produced by a feature implementation.
#[derive(Debug, Clone)]
pub struct FeatureSeries {
    name: String,
    values: Vec<f64>,
}

impl FeatureSeries {
    /// Create a new feature series.
    pub fn new(name: impl Into<String>, values: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            values,
        }
    }

    /// Name of the feature.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Borrow the raw feature values.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// Length of the feature series.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the feature contains no values.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Trait implemented by all feature engineering components.
pub trait Feature {
    /// Unique name of the feature.
    fn name(&self) -> &'static str;

    /// Compute the feature on top of the supplied market data.
    fn compute(&self, context: &FeatureContext<'_>) -> FeatureSeries;
}

/// Convenience wrapper around a collection of features.
#[derive(Default)]
pub struct FeatureRegistry {
    features: Vec<Box<dyn Feature + Send + Sync>>,
}

impl FeatureRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new feature.
    pub fn register<F>(&mut self, feature: F)
    where
        F: Feature + Send + Sync + 'static,
    {
        self.features.push(Box::new(feature));
    }

    /// Compute all registered features and collect them into a [`FeatureSet`].
    pub fn compute(&self, data: &HyperliquidData) -> FeatureSet {
        let context = FeatureContext::new(data);
        let mut series = Vec::with_capacity(self.features.len());
        for feature in &self.features {
            series.push(feature.compute(&context));
        }
        FeatureSet { series }
    }

    /// Number of registered features.
    pub fn len(&self) -> usize {
        self.features.len()
    }

    /// Whether no features have been registered.
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
}

/// Compute a feature set directly from an iterator of feature implementations.
///
/// This helper is handy when features are constructed ad-hoc instead of being
/// stored in a registry.
pub fn compute_feature_set<I>(data: &HyperliquidData, features: I) -> FeatureSet
where
    I: IntoIterator<Item = Box<dyn Feature + Send + Sync>>,
{
    let context = FeatureContext::new(data);
    let mut series = Vec::new();
    for feature in features {
        series.push(feature.compute(&context));
    }
    FeatureSet { series }
}

/// Collection of feature series that can be consumed by the alpha pipeline.
#[derive(Debug, Clone)]
pub struct FeatureSet {
    series: Vec<FeatureSeries>,
}

impl FeatureSet {
    /// Create a new feature set.
    pub fn new(series: Vec<FeatureSeries>) -> Self {
        Self { series }
    }

    /// Iterate over all feature series.
    pub fn iter(&self) -> impl Iterator<Item = &FeatureSeries> {
        self.series.iter()
    }

    /// Consume the set and return the inner vector.
    pub fn into_inner(self) -> Vec<FeatureSeries> {
        self.series
    }

    /// Number of features contained in the set.
    pub fn len(&self) -> usize {
        self.series.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.series.is_empty()
    }

    /// Fetch a feature by name.
    pub fn get(&self, name: &str) -> Option<&FeatureSeries> {
        self.series.iter().find(|series| series.name == name)
    }
}

/// Relative Strength Index feature implementation.
pub struct RsiFeature {
    /// Look-back window for the RSI calculation.
    pub period: usize,
}

impl Feature for RsiFeature {
    fn name(&self) -> &'static str {
        "RSI"
    }

    fn compute(&self, context: &FeatureContext<'_>) -> FeatureSeries {
        let period = self.period.max(1);
        let closes = &context.data().close;
        let mut values = Vec::with_capacity(closes.len());

        if closes.is_empty() {
            return FeatureSeries::new(self.name(), values);
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        values.push(f64::NAN); // first value has no delta
        for i in 1..=period.min(closes.len() - 1) {
            let delta = closes[i] - closes[i - 1];
            if delta >= 0.0 {
                gains += delta;
            } else {
                losses -= delta;
            }
            values.push(f64::NAN);
        }

        if closes.len() <= period {
            // Not enough data to compute RSI; fill with NaNs
            values.resize(closes.len(), f64::NAN);
            return FeatureSeries::new(self.name(), values);
        }

        let mut avg_gain = gains / period as f64;
        let mut avg_loss = losses / period as f64;

        let rsi_value = if avg_loss == 0.0 {
            100.0
        } else {
            let rs = avg_gain / avg_loss;
            100.0 - (100.0 / (1.0 + rs))
        };
        if values.len() > period {
            values[period] = rsi_value;
        }

        for i in (period + 1)..closes.len() {
            let delta = closes[i] - closes[i - 1];
            if delta >= 0.0 {
                avg_gain = ((avg_gain * (period as f64 - 1.0)) + delta) / period as f64;
                avg_loss = (avg_loss * (period as f64 - 1.0)) / period as f64;
            } else {
                avg_gain = (avg_gain * (period as f64 - 1.0)) / period as f64;
                avg_loss = ((avg_loss * (period as f64 - 1.0)) - delta) / period as f64;
            }

            let rsi = if avg_loss == 0.0 {
                100.0
            } else {
                let rs = avg_gain / avg_loss;
                100.0 - (100.0 / (1.0 + rs))
            };
            values.push(rsi);
        }

        FeatureSeries::new(self.name(), values)
    }
}

/// Rolling volatility feature computed using the population standard deviation of log returns.
pub struct VolatilityFeature {
    /// Rolling window length.
    pub window: usize,
}

impl Feature for VolatilityFeature {
    fn name(&self) -> &'static str {
        "VOLATILITY"
    }

    fn compute(&self, context: &FeatureContext<'_>) -> FeatureSeries {
        let closes = &context.data().close;
        let window = self.window.max(2);
        if closes.len() < 2 {
            return FeatureSeries::new(self.name(), vec![f64::NAN; closes.len()]);
        }

        let mut log_returns = Vec::with_capacity(closes.len() - 1);
        for w in closes.windows(2) {
            let prev = w[0];
            let current = w[1];
            log_returns.push((current / prev).ln());
        }

        let mut values = vec![f64::NAN; closes.len()];
        if log_returns.len() + 1 <= window {
            return FeatureSeries::new(self.name(), values);
        }

        for end in window..=log_returns.len() {
            let slice = &log_returns[end - window..end];
            let mean = slice.iter().sum::<f64>() / slice.len() as f64;
            let variance = slice
                .iter()
                .map(|value| {
                    let diff = value - mean;
                    diff * diff
                })
                .sum::<f64>()
                / slice.len() as f64;
            let std_dev = variance.sqrt();
            values[end] = std_dev;
        }

        FeatureSeries::new(self.name(), values)
    }
}

/// Simple lagged return feature using percentage returns.
pub struct LagReturnFeature {
    /// Number of periods to lag the return calculation.
    pub lag: usize,
}

impl Feature for LagReturnFeature {
    fn name(&self) -> &'static str {
        "LAG_RETURN"
    }

    fn compute(&self, context: &FeatureContext<'_>) -> FeatureSeries {
        let closes = &context.data().close;
        let lag = self.lag.max(1);
        if closes.len() <= lag {
            return FeatureSeries::new(self.name(), vec![f64::NAN; closes.len()]);
        }

        let mut values = vec![f64::NAN; closes.len()];
        for i in lag..closes.len() {
            let previous = closes[i - lag];
            let current = closes[i];
            if previous != 0.0 {
                values[i] = (current / previous) - 1.0;
            } else {
                values[i] = f64::NAN;
            }
        }

        FeatureSeries::new(self.name(), values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        for i in 0..10 {
            let price = 100.0 + i as f64;
            datetime.push(tz.timestamp_opt(i as i64, 0).unwrap());
            open.push(price - 0.5);
            high.push(price + 1.0);
            low.push(price - 1.0);
            close.push(price);
            volume.push(1_000.0 + i as f64 * 10.0);
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
    fn feature_registry_computes_all_features() {
        let data = mock_data();
        let mut registry = FeatureRegistry::new();
        registry.register(RsiFeature { period: 5 });
        registry.register(VolatilityFeature { window: 3 });
        registry.register(LagReturnFeature { lag: 1 });

        let feature_set = registry.compute(&data);
        assert_eq!(feature_set.len(), 3);
        assert!(feature_set
            .iter()
            .all(|series| series.len() == data.close.len()));
    }

    #[test]
    fn compute_feature_set_from_iterator() {
        let data = mock_data();
        let feature_set = compute_feature_set(
            &data,
            vec![
                boxed_feature(RsiFeature { period: 4 }),
                boxed_feature(VolatilityFeature { window: 4 }),
                boxed_feature(LagReturnFeature { lag: 2 }),
            ],
        );

        assert_eq!(feature_set.len(), 3);
        assert!(feature_set.get("RSI").is_some());
    }
}
