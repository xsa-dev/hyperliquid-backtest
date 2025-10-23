//! Genetic algorithm example built on the reusable optimization framework.
//!
//! This example demonstrates how strategies can express their parameters via the
//! [`Genome`](hyperliquid_backtest::optimization::Genome) trait and plug into the
//! [`GeneticOptimizer`](hyperliquid_backtest::optimization::GeneticOptimizer).
//! Instead of running a full backtest we rely on a synthetic scoring function to
//! keep the example lightweight and deterministic.

use anyhow::Result;
use hyperliquid_backtest::optimization::{
    FitnessEvaluator, GeneticOptimizer, GeneticOptimizerConfig, Genome, OptimizationOutcome,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Strategy parameters (our genome).
#[derive(Clone, Debug)]
struct SmaParams {
    fast: u32,
    slow: u32,
    risk: f64,
}

impl Genome for SmaParams {
    fn random(rng: &mut dyn rand::RngCore) -> Self {
        let mut fast = rng.gen_range(5..=40);
        let mut slow = rng.gen_range(20..=160);
        if slow <= fast {
            slow = fast + 5;
        }
        let risk = rng.gen_range(0.2..=2.0);
        Self { fast, slow, risk }
    }

    fn mutate(&mut self, rng: &mut dyn rand::RngCore) {
        if rng.gen_bool(0.4) {
            let delta: i32 = rng.gen_range(-3..=3);
            let new_fast = (self.fast as i32 + delta).clamp(5, 60);
            self.fast = new_fast as u32;
        }
        if rng.gen_bool(0.4) {
            let delta: i32 = rng.gen_range(-8..=8);
            let new_slow = (self.slow as i32 + delta).clamp(10, 200);
            self.slow = new_slow as u32;
        }
        if self.slow <= self.fast {
            self.slow = self.fast + 5;
        }
        if rng.gen_bool(0.3) {
            let delta = rng.gen_range(-0.2..=0.2);
            self.risk = (self.risk + delta).clamp(0.1, 3.0);
        }
    }

    fn crossover(&self, other: &Self, rng: &mut dyn rand::RngCore) -> Self {
        let fast = if rng.gen_bool(0.5) {
            self.fast
        } else {
            other.fast
        };
        let slow = if rng.gen_bool(0.5) {
            self.slow
        } else {
            other.slow
        };
        let risk = if rng.gen_bool(0.5) {
            self.risk
        } else {
            other.risk
        };
        let mut child = Self { fast, slow, risk };
        if child.slow <= child.fast {
            child.slow = child.fast + 5;
        }
        child
    }
}

/// Synthetic metrics returned by the evaluator.
#[derive(Clone, Debug)]
struct StrategyMetrics {
    total_return: f64,
    sharpe_ratio: f64,
    max_drawdown: f64,
}

/// Deterministic evaluator that mimics a backtest result.
struct SyntheticEvaluator;

impl FitnessEvaluator<SmaParams> for SyntheticEvaluator {
    type Metrics = StrategyMetrics;

    fn evaluate(
        &self,
        candidate: &SmaParams,
    ) -> Result<OptimizationOutcome<Self::Metrics>, Box<dyn std::error::Error + Send + Sync>> {
        let fast = candidate.fast as f64;
        let slow = candidate.slow as f64;
        let ratio = fast / slow;

        // Synthetic objective components.
        let total_return = 0.05 + 0.6 * (-(fast - 18.0).powi(2) / 600.0).exp();
        let sharpe = 1.0 + 0.8 * (-(slow - 90.0).powi(2) / 8000.0).exp();
        let drawdown_penalty = 0.12 + 0.5 * (ratio - 0.25).abs();
        let risk_penalty = (candidate.risk - 1.2).abs() * 0.1;

        let fitness = total_return * 0.7 + sharpe * 0.4 - drawdown_penalty * 0.8 - risk_penalty;

        Ok(OptimizationOutcome {
            fitness,
            metrics: StrategyMetrics {
                total_return,
                sharpe_ratio: sharpe,
                max_drawdown: drawdown_penalty,
            },
        })
    }
}

fn main() -> Result<()> {
    let config = GeneticOptimizerConfig {
        population_size: 48,
        elitism: 4,
        generations: 20,
        tournament_size: 3,
    };

    let optimizer = GeneticOptimizer::new(config, SyntheticEvaluator);
    let mut rng = StdRng::seed_from_u64(42);
    let result = optimizer.run(&mut rng)?;

    println!("Best candidate: {:?}", result.best_candidate);
    println!(
        "Metrics: return={:.4}, sharpe={:.4}, max_dd={:.4}",
        result.best_metrics.total_return,
        result.best_metrics.sharpe_ratio,
        result.best_metrics.max_drawdown
    );
    println!("Fitness: {:.4}", result.best_fitness);

    for generation in result.generations {
        println!(
            "Generation {:>2}: best={:.4}, avg={:.4}",
            generation.index, generation.best_fitness, generation.average_fitness
        );
    }

    Ok(())
}
