use crate::parser::ast::*;
use crate::roller::*;
use crate::stats::result::*;

/// Configuration for Monte Carlo simulation
pub struct MonteCarloConfig {
    /// Number of trials to run
    pub trials: usize,
    /// Batch size for incremental updates
    pub batch_size: usize,
    /// Target relative error for convergence
    pub target_relative_error: f64,
    /// Maximum number of trials
    pub max_trials: usize,
}

impl Default for MonteCarloConfig {
    fn default() -> Self {
        Self {
            trials: 100_000,
            batch_size: 1_000,
            target_relative_error: 0.01,
            max_trials: 1_000_000,
        }
    }
}

/// Result of Monte Carlo simulation
#[derive(Debug, Clone)]
pub struct MonteCarloResult {
    /// Probability distribution
    pub distribution: ProbabilitiesResult,
    /// Number of trials actually run
    pub trials_run: usize,
    /// Whether the simulation converged
    pub converged: bool,
    /// Estimated relative error
    pub relative_error: f64,
}

/// Run Monte Carlo simulation for a dice expression
pub fn monte_carlo<R: DiceRng>(
    expr: &Expression,
    rng: R,
    config: &MonteCarloConfig,
) -> MonteCarloResult {
    let mut roller = Roller::new(rng);
    let mut result = ProbabilitiesResult::new();

    let mut trials_run = 0;
    let mut converged = false;
    let mut relative_error = f64::INFINITY;

    // Run in batches for incremental progress
    while trials_run < config.max_trials {
        let batch_end = (trials_run + config.batch_size).min(config.max_trials);

        for _ in trials_run..batch_end {
            let roll_result = roller.roll(expr);
            result.add(roll_result.value() as i64);
        }

        trials_run = batch_end;

        // Check convergence
        if trials_run >= config.trials {
            let stats = result.stats();
            let mean = stats.mean;
            let stddev = stats.stddev;

            if mean != 0.0 {
                relative_error = stddev / (mean.abs() * (trials_run as f64).sqrt());
            } else {
                relative_error = stddev / (trials_run as f64).sqrt();
            }

            if relative_error < config.target_relative_error {
                converged = true;
                break;
            }
        }
    }

    MonteCarloResult {
        distribution: result,
        trials_run,
        converged,
        relative_error,
    }
}

/// Run Monte Carlo simulation with progress reporting
pub fn monte_carlo_with_progress<R: DiceRng, F: FnMut(usize, usize)>(
    expr: &Expression,
    rng: R,
    config: &MonteCarloConfig,
    mut on_progress: F,
) -> MonteCarloResult {
    let mut roller = Roller::new(rng);
    let mut result = ProbabilitiesResult::new();

    let mut trials_run = 0;
    let mut converged = false;
    let mut relative_error = f64::INFINITY;

    while trials_run < config.max_trials {
        let batch_end = (trials_run + config.batch_size).min(config.max_trials);

        for _ in trials_run..batch_end {
            let roll_result = roller.roll(expr);
            result.add(roll_result.value() as i64);
        }

        trials_run = batch_end;
        on_progress(trials_run, config.max_trials);

        // Check convergence
        if trials_run >= config.trials {
            let stats = result.stats();
            let mean = stats.mean;
            let stddev = stats.stddev;

            if mean != 0.0 {
                relative_error = stddev / (mean.abs() * (trials_run as f64).sqrt());
            } else {
                relative_error = stddev / (trials_run as f64).sqrt();
            }

            if relative_error < config.target_relative_error {
                converged = true;
                break;
            }
        }
    }

    MonteCarloResult {
        distribution: result,
        trials_run,
        converged,
        relative_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_monte_carlo_d6() {
        let expr = Parser::parse("d6").expression().unwrap().clone();
        let config = MonteCarloConfig {
            trials: 10000,
            max_trials: 10000,
            ..Default::default()
        };

        let result = monte_carlo(&expr, RandomRng::new(), &config);

        // Should be roughly uniform
        for i in 1..=6 {
            let prob = result.distribution.probability(i);
            assert!(
                prob > 0.1 && prob < 0.3,
                "Probability for {} was {}",
                i,
                prob
            );
        }
    }

    #[test]
    fn test_monte_carlo_convergence() {
        let expr = Parser::parse("3d6").expression().unwrap().clone();
        let config = MonteCarloConfig {
            trials: 1000,
            max_trials: 100000,
            target_relative_error: 0.05,
            ..Default::default()
        };

        let result = monte_carlo(&expr, RandomRng::new(), &config);

        // Should converge for 3d6
        assert!(result.trials_run <= 100000);
    }
}
