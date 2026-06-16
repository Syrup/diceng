use cap::Cap;
use clap::{Parser, Subcommand, ValueEnum};
use diceng::display::render_verbose;
use diceng::*;
use std::alloc;

#[global_allocator]
static ALLOCATOR: Cap<alloc::System> = Cap::new(alloc::System, usize::MAX);

#[derive(Parser)]
#[command(
    name = "diceng",
    about = "Fast dice expression parser, roller, and probability analyzer"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Roll a dice expression
    Roll {
        /// Dice expression (e.g., "4d6k3", "3d6e5")
        expression: String,
        /// Use a specific seed for reproducible rolls
        #[arg(long)]
        seed: Option<u32>,
        /// Output format
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
        /// Show detailed roll breakdown
        #[arg(short, long)]
        verbose: bool,
    },
    /// Compute probability distribution
    Stats {
        /// Dice expression
        expression: String,
        /// Number of Monte Carlo trials (if exact computation fails)
        #[arg(long, default_value = "100000")]
        trials: usize,
        /// Output format
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
        /// Number of decimal places for probabilities
        #[arg(long, default_value = "4")]
        precision: usize,
    },
    /// Validate a dice expression
    Check { expression: String },
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

fn main() {
    // Set 1GB memory limit as safety net
    const ONE_GB: usize = 1024 * 1024 * 1024;
    ALLOCATOR.set_limit(ONE_GB).unwrap();

    let cli = Cli::parse();

    match cli.command {
        Command::Roll {
            expression,
            seed,
            format,
            verbose,
        } => {
            handle_roll(&expression, seed, format, verbose);
        }
        Command::Stats {
            expression,
            trials,
            format,
            precision,
        } => {
            handle_stats(&expression, trials, format, precision);
        }
        Command::Check { expression } => {
            handle_check(&expression);
        }
    }
}

fn handle_roll(expression: &str, seed: Option<u32>, format: OutputFormat, verbose: bool) {
    let result = parse(expression);

    if !result.success() {
        let errors: Vec<String> = result
            .errors()
            .iter()
            .map(|e| format!("  - Position {}: {}", e.position, e.message))
            .collect();
        eprintln!("Parse errors:\n{}", errors.join("\n"));
        std::process::exit(1);
    }

    let expr = result.expression().unwrap();

    let roll_result = match seed {
        Some(s) => roll_seeded(expr, s),
        None => roll(expr),
    };

    match format {
        OutputFormat::Text => {
            if verbose {
                let entries = roll_result.to_verbose_entries();
                let output = render_verbose(expression, roll_result.value(), &entries);
                print!("{}", output);
                if let Some(s) = seed {
                    println!("Seed: {}", s);
                }
            } else {
                println!("{}", roll_result.value());
            }
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "expression": expression,
                "seed": seed,
                "result": roll_result.value(),
                "dice": roll_result.dice_values(),
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
    }
}

fn handle_stats(expression: &str, trials: usize, format: OutputFormat, precision: usize) {
    let result = parse(expression);

    if !result.success() {
        let errors: Vec<String> = result
            .errors()
            .iter()
            .map(|e| format!("  - Position {}: {}", e.position, e.message))
            .collect();
        eprintln!("Parse errors:\n{}", errors.join("\n"));
        std::process::exit(1);
    }

    let expr = result.expression().unwrap();

    // Try exact first, fall back to Monte Carlo
    let (dist, is_exact, trials_run) = match exact_distribution(expr) {
        Some(exact_dist) => (exact_dist, true, 0),
        None => {
            let mc_config = MonteCarloConfig {
                trials,
                max_trials: trials,
                ..Default::default()
            };
            let mc_result = monte_carlo(expr, RandomRng::new(), &mc_config);
            (mc_result.distribution, false, mc_result.trials_run)
        }
    };

    let stats = dist.stats();

    match format {
        OutputFormat::Text => {
            println!("Expression: {}", expression);
            println!("Method: {}", if is_exact { "Exact" } else { "Monte Carlo" });
            if !is_exact {
                println!("Trials: {}", trials_run);
            }
            println!();
            println!("Min: {}", stats.min);
            println!("Max: {}", stats.max);
            println!("Mean: {:.prec$}", stats.mean, prec = precision);
            println!("Stddev: {:.prec$}", stats.stddev, prec = precision);
            println!();

            // Distribution table
            println!("Distribution:");
            println!(
                "{:>8} {:>10} {:>10} {:>10} {:>10}",
                "Value", "Prob", "At Least", "At Most", "Count"
            );
            println!("{}", "-".repeat(52));

            for &(value, prob) in &stats.distribution {
                let count = dist.distribution.get(&value).unwrap_or(&0);
                let at_least = dist.reverse_cumulative_probability(value);
                let at_most = dist.cumulative_probability(value);

                println!(
                    "{:>8} {:>10.4} {:>10.4} {:>10.4} {:>10}",
                    value, prob, at_least, at_most, count
                );
            }
        }
        OutputFormat::Json => {
            let distribution: Vec<serde_json::Value> = stats
                .distribution
                .iter()
                .map(|&(value, prob)| {
                    serde_json::json!({
                        "value": value,
                        "probability": prob,
                        "at_least": dist.reverse_cumulative_probability(value),
                        "at_most": dist.cumulative_probability(value),
                        "count": dist.distribution.get(&value).unwrap_or(&0),
                    })
                })
                .collect();

            let output = serde_json::json!({
                "expression": expression,
                "method": if is_exact { "exact" } else { "monte_carlo" },
                "trials": if is_exact { serde_json::Value::Null } else { serde_json::json!(trials_run) },
                "min": stats.min,
                "max": stats.max,
                "mean": stats.mean,
                "stddev": stats.stddev,
                "distribution": distribution,
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
    }
}

fn handle_check(expression: &str) {
    let result = parse(expression);

    if result.success() {
        println!("✓ Valid expression: {}", expression);
    } else {
        println!("✗ Invalid expression: {}", expression);
        for error in result.errors() {
            println!("  - Position {}: {}", error.position, error.message);
            if let Some(ref suggestion) = error.suggestion {
                println!("    Suggestion: {}", suggestion);
            }
        }
        std::process::exit(1);
    }
}
