use rand::RngCore;
use std::{fmt, marker::PhantomData};

/// Defines how candidate parameters behave within the genetic algorithm.
pub trait Genome: Clone + Send + Sync + Sized {
    /// Generate a random candidate.
    fn random(rng: &mut dyn RngCore) -> Self;

    /// Produce a mutated version of this candidate.
    fn mutate(&mut self, rng: &mut dyn RngCore);

    /// Combine the candidate with another one to create an offspring.
    fn crossover(&self, other: &Self, rng: &mut dyn RngCore) -> Self;
}

/// Outcome of evaluating a candidate.
#[derive(Debug, Clone)]
pub struct OptimizationOutcome<M> {
    /// Fitness score produced by the evaluation function. Higher is better.
    pub fitness: f64,
    /// Additional metrics reported by the evaluator.
    pub metrics: M,
}

/// Error returned by the optimizer when it cannot produce a valid result.
#[derive(thiserror::Error, Debug)]
pub enum OptimizationError {
    /// Returned when the population size is zero.
    #[error("population size must be greater than zero")]
    EmptyPopulation,
    /// Returned when elitism is equal to or greater than the population size.
    #[error("elitism must be smaller than the population size")]
    InvalidElitism,
    /// Returned when the tournament size is zero.
    #[error("tournament size must be greater than zero")]
    InvalidTournamentSize,
    /// Returned when evaluating a candidate fails.
    #[error("candidate evaluation failed: {0}")]
    EvaluationFailed(String),
}

/// Result of an optimization run.
#[derive(Debug, Clone)]
pub struct OptimizationResult<G, M>
where
    G: Genome,
    M: Clone + Send + Sync,
{
    /// Best candidate discovered by the optimizer.
    pub best_candidate: G,
    /// Metrics associated with the best candidate.
    pub best_metrics: M,
    /// Fitness score of the best candidate.
    pub best_fitness: f64,
    /// Summary statistics for every processed generation.
    pub generations: Vec<GenerationSummary<M>>,
}

/// Summary of a processed generation.
#[derive(Debug, Clone)]
pub struct GenerationSummary<M>
where
    M: Clone + Send + Sync,
{
    /// Generation index starting from zero.
    pub index: usize,
    /// Best fitness score observed in the generation.
    pub best_fitness: f64,
    /// Average fitness across the generation.
    pub average_fitness: f64,
    /// Metrics produced by the best candidate of the generation.
    pub best_metrics: M,
}

/// Configuration for the genetic optimizer.
#[derive(Debug, Clone, Copy)]
pub struct GeneticOptimizerConfig {
    /// Number of individuals in the population.
    pub population_size: usize,
    /// Number of elite individuals copied verbatim to the next generation.
    pub elitism: usize,
    /// Number of generations to process.
    pub generations: usize,
    /// Tournament size used for parent selection.
    pub tournament_size: usize,
}

impl Default for GeneticOptimizerConfig {
    fn default() -> Self {
        Self {
            population_size: 32,
            elitism: 2,
            generations: 20,
            tournament_size: 3,
        }
    }
}

/// Evaluation function used by the optimizer.
pub trait FitnessEvaluator<G>: Send + Sync
where
    G: Genome,
{
    /// Additional metrics reported for each candidate.
    type Metrics: Clone + Send + Sync;

    /// Evaluate the provided candidate.
    fn evaluate(
        &self,
        candidate: &G,
    ) -> Result<OptimizationOutcome<Self::Metrics>, Box<dyn std::error::Error + Send + Sync>>;
}

impl<G, M, F, E> FitnessEvaluator<G> for F
where
    G: Genome,
    M: Clone + Send + Sync + 'static,
    F: Fn(&G) -> Result<OptimizationOutcome<M>, E> + Send + Sync,
    E: std::error::Error + Send + Sync + 'static,
{
    type Metrics = M;

    fn evaluate(
        &self,
        candidate: &G,
    ) -> Result<OptimizationOutcome<M>, Box<dyn std::error::Error + Send + Sync>> {
        self(candidate).map_err(|err| Box::new(err) as _)
    }
}

#[derive(Clone)]
struct Individual<G, M>
where
    G: Genome,
    M: Clone + Send + Sync,
{
    genome: G,
    metrics: Option<M>,
    fitness: f64,
}

impl<G, M> Individual<G, M>
where
    G: Genome,
    M: Clone + Send + Sync,
{
    fn unevaluated(genome: G) -> Self {
        Self {
            genome,
            metrics: None,
            fitness: f64::NEG_INFINITY,
        }
    }
}

/// Simple, framework-agnostic genetic algorithm optimizer.
pub struct GeneticOptimizer<G, E>
where
    G: Genome,
    E: FitnessEvaluator<G>,
{
    config: GeneticOptimizerConfig,
    evaluator: E,
    phantom: PhantomData<G>,
}

impl<G, E> GeneticOptimizer<G, E>
where
    G: Genome,
    E: FitnessEvaluator<G>,
{
    /// Create a new optimizer.
    pub fn new(config: GeneticOptimizerConfig, evaluator: E) -> Self {
        Self {
            config,
            evaluator,
            phantom: PhantomData,
        }
    }

    /// Execute the optimization run and return the best candidate discovered.
    pub fn run<R>(
        &self,
        rng: &mut R,
    ) -> Result<OptimizationResult<G, E::Metrics>, OptimizationError>
    where
        R: RngCore,
    {
        if self.config.population_size == 0 {
            return Err(OptimizationError::EmptyPopulation);
        }

        if self.config.elitism >= self.config.population_size {
            return Err(OptimizationError::InvalidElitism);
        }

        if self.config.tournament_size == 0 {
            return Err(OptimizationError::InvalidTournamentSize);
        }

        let mut population: Vec<Individual<G, E::Metrics>> = (0..self.config.population_size)
            .map(|_| Individual::unevaluated(G::random(rng)))
            .collect();

        let mut generation_summaries = Vec::with_capacity(self.config.generations + 1);

        self.evaluate_population(&mut population)?;
        population.sort_by(|a, b| b.fitness.total_cmp(&a.fitness));
        generation_summaries.push(Self::summarize_generation(0, &population));

        for generation in 1..=self.config.generations {
            let mut next_population: Vec<Individual<G, E::Metrics>> =
                Vec::with_capacity(self.config.population_size);
            next_population.extend(population.iter().take(self.config.elitism).cloned());

            while next_population.len() < self.config.population_size {
                let parent_a =
                    Self::tournament_select(&population, self.config.tournament_size, rng);
                let parent_b =
                    Self::tournament_select(&population, self.config.tournament_size, rng);

                let mut child_genome = parent_a.genome.crossover(&parent_b.genome, rng);
                child_genome.mutate(rng);
                next_population.push(Individual::unevaluated(child_genome));
            }

            population = next_population;
            self.evaluate_population(&mut population)?;
            population.sort_by(|a, b| b.fitness.total_cmp(&a.fitness));
            generation_summaries.push(Self::summarize_generation(generation, &population));
        }

        let best = population
            .first()
            .expect("population cannot be empty after initialization");

        Ok(OptimizationResult {
            best_candidate: best.genome.clone(),
            best_metrics: best
                .metrics
                .clone()
                .expect("metrics must be present after evaluation"),
            best_fitness: best.fitness,
            generations: generation_summaries,
        })
    }

    fn evaluate_population(
        &self,
        population: &mut [Individual<G, E::Metrics>],
    ) -> Result<(), OptimizationError> {
        for individual in population.iter_mut() {
            if individual.metrics.is_some() {
                continue;
            }

            let outcome = self
                .evaluator
                .evaluate(&individual.genome)
                .map_err(|err| OptimizationError::EvaluationFailed(err.to_string()))?;

            individual.fitness = if outcome.fitness.is_finite() {
                outcome.fitness
            } else {
                f64::NEG_INFINITY
            };
            individual.metrics = Some(outcome.metrics);
        }

        Ok(())
    }

    fn tournament_select<'a, R>(
        population: &'a [Individual<G, E::Metrics>],
        tournament_size: usize,
        rng: &mut R,
    ) -> &'a Individual<G, E::Metrics>
    where
        R: RngCore,
    {
        let mut best_index = rng.next_u32() as usize % population.len();
        let mut best_fitness = population[best_index].fitness;

        for _ in 1..tournament_size {
            let idx = rng.next_u32() as usize % population.len();
            let fitness = population[idx].fitness;
            if fitness > best_fitness {
                best_index = idx;
                best_fitness = fitness;
            }
        }

        &population[best_index]
    }

    fn summarize_generation(
        index: usize,
        population: &[Individual<G, E::Metrics>],
    ) -> GenerationSummary<E::Metrics> {
        let mut total = 0.0;
        let mut count = 0usize;
        let mut best_fitness = f64::NEG_INFINITY;
        let mut best_metrics = None;

        for individual in population {
            if individual.fitness > best_fitness {
                best_fitness = individual.fitness;
                best_metrics = individual.metrics.clone();
            }

            if individual.fitness.is_finite() {
                total += individual.fitness;
                count += 1;
            }
        }

        let average = if count > 0 {
            total / count as f64
        } else {
            f64::NEG_INFINITY
        };

        GenerationSummary {
            index,
            best_fitness,
            average_fitness: average,
            best_metrics: best_metrics.expect("metrics must exist after evaluation"),
        }
    }
}

impl<G, E> fmt::Debug for GeneticOptimizer<G, E>
where
    G: Genome,
    E: FitnessEvaluator<G>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GeneticOptimizer")
            .field("config", &self.config)
            .finish()
    }
}
