// examples/ga_optimize.rs
// GA-оптимизатор гиперпараметров поверх hyperliquid-backtest.
//
// Оптимизируем параметры простой стратегии (например, enhanced_sma_cross):
//  - fast_ma: u32   [5..50]
//  - slow_ma: u32   [10..150], slow > fast
//  - risk_mult: f64 [0.5..3.0] — условный множитель риска/позиции (пример)
//
// Цель: максимизировать Total Return и Sharpe, минимизировать Max Drawdown.
// Для простоты — скалируем в единый скоринг (можно заменить на Pareto/NSGA-II позже).
//
// Требования:
//   cargo run --example ga_optimize
//
// Основано на рабочем примере получения данных и бэктеста из README/docs.
// См. API: prelude, HyperliquidData::with_ohlc_data, HyperliquidBacktest, enhanced_sma_cross.
// Docs: https://docs.rs/hyperliquid-backtest (см. prelude, Quick Start) и README репозитория.
//

use anyhow::{Context, Result};
use chrono::{Duration, FixedOffset, TimeZone, Utc};
use hyperliquid_backtest::prelude::*;
use hyperliquid_backtest::{
    backtest::{HyperliquidBacktest, HyperliquidCommission},
    data::HyperliquidData,
    errors::HyperliquidBacktestError,
    strategies::{enhanced_sma_cross, FundingAwareConfig},
};
use hyperliquid_rust_sdk::{types::Candle, BaseUrl, InfoClient};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;

// -------------------------------
// Параметры стратегии (хромосома)
// -------------------------------
#[derive(Clone, Debug)]
struct Params {
    fast_ma: u32,
    slow_ma: u32,
    risk_mult: f64,
}

impl Params {
    fn random<R: Rng>(rng: &mut R) -> Self {
        let mut fast = rng.gen_range(5..=50);
        let mut slow = rng.gen_range(10..=150);
        if slow <= fast {
            slow = fast + rng.gen_range(5..=50).min(150 - fast);
        }
        let risk_mult = rng.gen_range(0.5..=3.0);
        Self {
            fast_ma: fast,
            slow_ma: slow.max(fast + 1),
            risk_mult,
        }
    }

    fn mutate<R: Rng>(&mut self, rng: &mut R) {
        if rng.gen_bool(0.3) {
            let delta: i32 = rng.gen_range(-5..=5);
            self.fast_ma = (self.fast_ma as i32 + delta).clamp(5, 80) as u32;
        }
        if rng.gen_bool(0.3) {
            let delta: i32 = rng.gen_range(-10..=10);
            self.slow_ma = (self.slow_ma as i32 + delta).clamp(10, 200) as u32;
        }
        if self.slow_ma <= self.fast_ma {
            self.slow_ma = self.fast_ma + rng.gen_range(1..=10);
        }
        if rng.gen_bool(0.3) {
            let delta = rng.gen_range(-0.3..=0.3);
            self.risk_mult = (self.risk_mult + delta).clamp(0.3, 5.0);
        }
    }

    fn crossover<R: Rng>(&self, other: &Self, rng: &mut R) -> Self {
        let fast = if rng.gen_bool(0.5) {
            self.fast_ma
        } else {
            other.fast_ma
        };
        let slow = if rng.gen_bool(0.5) {
            self.slow_ma
        } else {
            other.slow_ma
        };
        let risk = if rng.gen_bool(0.5) {
            self.risk_mult
        } else {
            other.risk_mult
        };
        let mut child = Self {
            fast_ma: fast,
            slow_ma: slow,
            risk_mult: risk,
        };
        if child.slow_ma <= child.fast_ma {
            child.slow_ma = child.fast_ma + 1;
        }
        child
    }
}

// -------------------------------
// Метрики fitness
// -------------------------------
#[derive(Clone, Debug)]
struct Metrics {
    total_return: f64, // 0.25 = 25%
    sharpe: f64,
    max_drawdown: f64, // положительное число, напр. 0.12 = 12%
}

fn score(m: &Metrics) -> f64 {
    let w1 = 1.0;
    let w2 = 0.8;
    let w3 = 0.7;
    w1 * m.total_return + w2 * m.sharpe - w3 * m.max_drawdown
}

// -------------------------------------------
// Запуск одного бэктеста и получение метрик
// -------------------------------------------
fn evaluate_once(
    base_data: &HyperliquidData,
    base_currency: &str,
    params: &Params,
    initial_capital: f64,
) -> Result<Metrics, HyperliquidBacktestError> {
    let mut data = base_data.clone();
    data.symbol = base_currency.to_string();

    let strategy = enhanced_sma_cross(
        data.to_rs_backtester_data(),
        params.fast_ma as usize,
        params.slow_ma as usize,
        FundingAwareConfig::default(),
    );

    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        initial_capital * params.risk_mult,
        HyperliquidCommission::default(),
    )?;

    backtest.initialize_base_backtest()?;
    backtest.calculate_with_funding()?;

    let report = backtest.enhanced_report()?;

    Ok(Metrics {
        total_return: report.total_return,
        sharpe: report.sharpe_ratio,
        max_drawdown: report.max_drawdown.abs(),
    })
}

// -------------------------------
// Простой GA-движок
// -------------------------------
#[derive(Clone)]
struct Individual {
    params: Params,
    metrics: Option<Metrics>,
    fitness: f64,
}

fn tournament<'a>(population: &'a [Individual], rng: &mut StdRng, k: usize) -> &'a Individual {
    let mut best = rng.gen_range(0..population.len());
    let mut best_score = population[best].fitness;
    for _ in 1..k {
        let idx = rng.gen_range(0..population.len());
        if population[idx].fitness > best_score {
            best = idx;
            best_score = population[idx].fitness;
        }
    }
    &population[best]
}

fn evaluate_population(
    population: &mut [Individual],
    base_data: &HyperliquidData,
    base_currency: &str,
    initial_capital: f64,
) -> Result<(), HyperliquidBacktestError> {
    population
        .par_iter_mut()
        .try_for_each(|ind| -> Result<(), HyperliquidBacktestError> {
            let metrics = evaluate_once(base_data, base_currency, &ind.params, initial_capital)?;
            ind.fitness = score(&metrics);
            ind.metrics = Some(metrics);
            Ok(())
        })
}

fn candles_to_data(
    candles: &[Candle],
    symbol: &str,
) -> Result<HyperliquidData, HyperliquidBacktestError> {
    let mut datetime = Vec::with_capacity(candles.len());
    let mut open = Vec::with_capacity(candles.len());
    let mut high = Vec::with_capacity(candles.len());
    let mut low = Vec::with_capacity(candles.len());
    let mut close = Vec::with_capacity(candles.len());
    let mut volume = Vec::with_capacity(candles.len());

    let tz = FixedOffset::east_opt(0).unwrap();

    for candle in candles {
        let ts = Utc
            .timestamp_millis_opt(candle.time_open as i64)
            .single()
            .ok_or_else(|| {
                HyperliquidBacktestError::conversion_error(format!(
                    "Invalid timestamp: {}",
                    candle.time_open
                ))
            })?;

        datetime.push(ts.with_timezone(&tz));
        open.push(candle.open.parse::<f64>().unwrap_or(0.0));
        high.push(candle.high.parse::<f64>().unwrap_or(0.0));
        low.push(candle.low.parse::<f64>().unwrap_or(0.0));
        close.push(candle.close.parse::<f64>().unwrap_or(0.0));
        volume.push(candle.vlm.parse::<f64>().unwrap_or(0.0));
    }

    HyperliquidData::with_ohlc_data(symbol.to_string(), datetime, open, high, low, close, volume)
}

async fn fetch_candles() -> Result<Vec<Candle>> {
    let client = InfoClient::new(None, Some(BaseUrl::Mainnet))
        .await
        .context("Failed to create Hyperliquid InfoClient")?;
    let end = Utc::now();
    let start = end - Duration::days(14);

    let candles = client
        .candles_snapshot(
            "BTC".to_string(),
            "1h".to_string(),
            start.timestamp_millis() as u64,
            end.timestamp_millis() as u64,
        )
        .await
        .context("Failed to fetch candles snapshot")?;

    if candles.is_empty() {
        anyhow::bail!("No candles received from Hyperliquid API");
    }

    Ok(candles)
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();

    println!("🔍 Fetching BTC/USDC 1h candles for the last 14 days...");
    let candles = fetch_candles().await?;
    println!("   ✅ Loaded {} candles", candles.len());

    let base_currency = "BTC";
    let base_data = candles_to_data(&candles, base_currency)?;

    let pop_size = 64usize;
    let elitism = 4usize;
    let generations = 25usize;
    let initial_capital = 10_000.0_f64;
    let seed = 42u64;

    let mut rng = StdRng::seed_from_u64(seed);

    let mut population: Vec<Individual> = (0..pop_size)
        .map(|_| Individual {
            params: Params::random(&mut rng),
            metrics: None,
            fitness: f64::NEG_INFINITY,
        })
        .collect();

    evaluate_population(&mut population, &base_data, base_currency, initial_capital)
        .context("Failed to evaluate initial population")?;

    population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
    println!(
        "Gen 0 | best score={:.4}, params={:?}, metrics={:?}",
        population[0].fitness, population[0].params, population[0].metrics
    );

    for gen in 1..=generations {
        let mut next: Vec<Individual> = population.iter().take(elitism).cloned().collect();

        while next.len() < pop_size {
            let parent1 = tournament(&population, &mut rng, 3);
            let parent2 = tournament(&population, &mut rng, 3);
            let mut child = Individual {
                params: parent1.params.crossover(&parent2.params, &mut rng),
                metrics: None,
                fitness: f64::NEG_INFINITY,
            };
            child.params.mutate(&mut rng);
            next.push(child);
        }

        evaluate_population(&mut next, &base_data, base_currency, initial_capital)
            .with_context(|| format!("Failed to evaluate generation {}", gen))?;

        next.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
        population = next;

        let best = &population[0];
        println!(
            "Gen {} | best score={:.4}, params={:?}, metrics={:?}",
            gen, best.fitness, best.params, best.metrics
        );
    }

    let best = &population[0];
    println!("\n=== BEST ===");
    println!("Score:   {:.6}", best.fitness);
    println!("Params:  {:?}", best.params);
    println!("Metrics: {:?}", best.metrics);

    Ok(())
}
