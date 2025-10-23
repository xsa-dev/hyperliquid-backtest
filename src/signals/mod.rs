//! Signal generation utilities built on top of evaluated alphas.
//!
//! The signal layer converts [`AlphaEvaluation`](crate::alpha::AlphaEvaluation)
//! outputs into actionable trading directives that can be consumed by
//! strategies or backtesting logic.

use crate::alpha::AlphaEvaluation;

/// Discrete trading instruction produced by a signal generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalValue {
    /// Take or maintain a long position.
    Long,
    /// Take or maintain a short position.
    Short,
    /// Stay flat.
    Flat,
}

impl SignalValue {
    /// Convert the signal into a signed position representation.
    pub fn as_position(&self) -> f64 {
        match self {
            SignalValue::Long => 1.0,
            SignalValue::Short => -1.0,
            SignalValue::Flat => 0.0,
        }
    }
}

/// Trait implemented by all signal generators.
pub trait SignalGenerator {
    /// Convert an [`AlphaEvaluation`] into a stream of trading signals.
    fn generate(&self, evaluation: &AlphaEvaluation) -> Vec<SignalValue>;
}

/// Signal generator using a symmetric threshold on normalised feature scores.
#[derive(Debug, Clone, Copy)]
pub struct ThresholdSignal {
    /// Threshold applied to the normalised scores.
    pub threshold: f64,
}

impl SignalGenerator for ThresholdSignal {
    fn generate(&self, evaluation: &AlphaEvaluation) -> Vec<SignalValue> {
        let threshold = self.threshold.abs();
        evaluation
            .scores
            .iter()
            .map(|score| {
                if !score.is_finite() || score.abs() < threshold {
                    SignalValue::Flat
                } else if *score > 0.0 {
                    SignalValue::Long
                } else {
                    SignalValue::Short
                }
            })
            .collect()
    }
}

/// Signal generator that buckets continuous scores into custom ranges.
#[derive(Debug, Clone)]
pub struct BucketSignal {
    /// Threshold that separates flat and directional states.
    pub neutral_threshold: f64,
    /// Upper bound after which the signal is considered strongly directional.
    pub aggressive_threshold: f64,
}

impl SignalGenerator for BucketSignal {
    fn generate(&self, evaluation: &AlphaEvaluation) -> Vec<SignalValue> {
        let neutral = self.neutral_threshold.abs();
        let aggressive = self.aggressive_threshold.abs().max(neutral);
        evaluation
            .scores
            .iter()
            .map(|score| {
                if !score.is_finite() || score.abs() < neutral {
                    SignalValue::Flat
                } else if score.abs() >= aggressive {
                    if *score > 0.0 {
                        SignalValue::Long
                    } else {
                        SignalValue::Short
                    }
                } else if *score > 0.0 {
                    SignalValue::Long
                } else {
                    SignalValue::Short
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alpha::AlphaEvaluation;

    fn mock_evaluation() -> AlphaEvaluation {
        AlphaEvaluation {
            feature_name: "mock".to_string(),
            model_name: "mock_model".to_string(),
            ic: 0.1,
            sharpe: 1.2,
            mean_return: 0.01,
            scores: vec![-1.5, -0.4, 0.0, 0.3, 0.8],
            ic_series: vec![0.0; 5],
            sample_size: 5,
        }
    }

    #[test]
    fn threshold_signal_generates_expected_values() {
        let evaluation = mock_evaluation();
        let generator = ThresholdSignal { threshold: 0.5 };
        let signals = generator.generate(&evaluation);
        assert_eq!(signals.len(), evaluation.scores.len());
        assert_eq!(signals[0], SignalValue::Short);
        assert_eq!(signals[1], SignalValue::Flat);
        assert_eq!(signals[4], SignalValue::Long);
    }

    #[test]
    fn bucket_signal_handles_multiple_levels() {
        let evaluation = mock_evaluation();
        let generator = BucketSignal {
            neutral_threshold: 0.2,
            aggressive_threshold: 1.0,
        };
        let signals = generator.generate(&evaluation);
        assert_eq!(signals[0], SignalValue::Short);
        assert_eq!(signals[2], SignalValue::Flat);
        assert_eq!(signals[4], SignalValue::Long);
    }
}
