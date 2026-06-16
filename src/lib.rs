pub mod display;
pub mod parser;
pub mod roller;
pub mod stats;
pub mod types;

pub use parser::*;
pub use roller::*;
pub use stats::*;
pub use types::*;

/// Parse a dice expression string into an AST.
///
/// Handles standard dice notation (`3d6`, `d20`, `d%`), shorthands
/// (`4d6k3`, `3d6!`, `2d6ro1`), and arithmetic (`2d6 + 4`).
///
/// Returns a [`ParseResult`] that either holds the parsed expression
/// or a list of errors with position information.
///
/// ```
/// use diceng::parse;
///
/// let result = parse("4d6k3");
/// assert!(result.success());
///
/// let expr = result.expression().unwrap();
/// ```
pub fn parse(input: &str) -> ParseResult {
    Parser::parse(input)
}

/// Roll a dice expression using the system's random source.
///
/// Each call produces a different result. For reproducible rolls,
/// use [`roll_seeded`] instead.
///
/// ```
/// use diceng::{parse, roll};
///
/// let expr = parse("3d6").expression().unwrap().clone();
/// let result = roll(&expr);
/// assert!(result.value() >= 3 && result.value() <= 18);
/// ```
pub fn roll(expr: &Expression) -> RollResult {
    let mut roller = Roller::new(RandomRng::new());
    roller.roll(expr)
}

/// Roll a dice expression with a deterministic seed.
///
/// Same seed always produces the same sequence of rolls. Useful for
/// testing, replays, or sharing roll results.
///
/// ```
/// use diceng::{parse, roll_seeded};
///
/// let expr = parse("4d6k3").expression().unwrap().clone();
/// let r1 = roll_seeded(&expr, 42);
/// let r2 = roll_seeded(&expr, 42);
/// assert_eq!(r1.value(), r2.value());
/// ```
pub fn roll_seeded(expr: &Expression, seed: u32) -> RollResult {
    let mut roller = Roller::new(LehmerRng::new(seed));
    roller.roll(expr)
}

/// Compute exact probability distribution via convolution and DP.
///
/// Returns `None` when the expression is too complex for exact computation
/// (e.g., emphasis rolls, very large dice pools). In that case, fall back
/// to [`monte_carlo_distribution`].
///
/// ```
/// use diceng::{parse, exact_distribution};
///
/// let expr = parse("2d6").expression().unwrap().clone();
/// let dist = exact_distribution(&expr).unwrap();
/// assert_eq!(dist.total, 36);
/// ```
pub fn exact_distribution(expr: &Expression) -> Option<ProbabilitiesResult> {
    exact::compute_exact(expr)
}

/// Estimate probability distribution using Monte Carlo simulation.
///
/// Runs `trials` random rolls and counts the outcomes. Useful when
/// exact computation fails or would use too much memory.
///
/// ```
/// use diceng::{parse, monte_carlo_distribution};
///
/// let expr = parse("2d6").expression().unwrap().clone();
/// let dist = monte_carlo_distribution(&expr, 10_000);
/// assert!(dist.total >= 9_000); // close to 10_000
/// ```
pub fn monte_carlo_distribution(expr: &Expression, trials: usize) -> ProbabilitiesResult {
    let config = MonteCarloConfig {
        trials,
        max_trials: trials,
        ..Default::default()
    };
    let result = monte_carlo::monte_carlo(expr, RandomRng::new(), &config);
    result.distribution
}

/// Compute probability distribution, picking the best method automatically.
///
/// Tries exact computation first. If that returns `None`, falls back to
/// Monte Carlo with the given number of trials.
///
/// ```
/// use diceng::{parse, compute_distribution};
///
/// let expr = parse("2d6").expression().unwrap().clone();
/// let dist = compute_distribution(&expr, 10_000);
/// assert_eq!(dist.total, 36); // exact for 2d6
/// ```
pub fn compute_distribution(expr: &Expression, monte_carlo_trials: usize) -> ProbabilitiesResult {
    // Try exact first
    if let Some(exact_dist) = exact_distribution(expr) {
        return exact_dist;
    }

    // Fall back to Monte Carlo
    monte_carlo_distribution(expr, monte_carlo_trials)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_roll() {
        let result = parse("3d6");
        assert!(result.success());

        let expr = result.expression().unwrap();
        let roll_result = roll(expr);
        assert!(roll_result.value() >= 3 && roll_result.value() <= 18);
    }

    #[test]
    fn test_seeded_roll() {
        let result = parse("4d6");
        assert!(result.success());

        let expr = result.expression().unwrap();
        let roll1 = roll_seeded(expr, 42);
        let roll2 = roll_seeded(expr, 42);

        assert_eq!(roll1.value(), roll2.value());
    }

    #[test]
    fn test_exact_distribution() {
        let result = parse("d6");
        assert!(result.success());

        let expr = result.expression().unwrap();
        let dist = exact_distribution(expr).unwrap();

        assert_eq!(dist.total, 6);
        for i in 1..=6 {
            assert!((dist.probability(i) - 1.0 / 6.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_compute_distribution() {
        let result = parse("2d6");
        assert!(result.success());

        let expr = result.expression().unwrap();
        let dist = compute_distribution(expr, 10000);

        // 2d6 has 36 total outcomes
        assert_eq!(dist.total, 36);
    }

    // Tests for new standard RPG notation features

    #[test]
    fn test_bang_explode_notation() {
        let result = parse("3d6!");
        assert!(result.success(), "3d6! should parse");
        let expr = result.expression().unwrap();
        // Use seeded roll for deterministic verification
        let roll_result = roll_seeded(expr, 42);
        assert!(roll_result.value() >= 3, "3d6! value should be >= 3");
        // Verify explosion can exceed normal max (18 for 3d6)
        // Try multiple seeds to find one that explodes
        let mut found_explode = false;
        for seed in 1..100 {
            let r = roll_seeded(expr, seed);
            if r.value() > 18 {
                found_explode = true;
                break;
            }
        }
        assert!(
            found_explode,
            "3d6! should sometimes exceed 18 (normal 3d6 max) due to explosions"
        );
    }

    #[test]
    fn test_bang_compound_notation() {
        let result = parse("3d6!!");
        assert!(result.success(), "3d6!! should parse");
        let expr = result.expression().unwrap();
        // Use seeded roll for deterministic verification
        let roll_result = roll_seeded(expr, 42);
        assert!(roll_result.value() >= 3, "3d6!! value should be >= 3");
        // Verify compound can exceed normal max (18 for 3d6)
        let mut found_compound = false;
        for seed in 1..100 {
            let r = roll_seeded(expr, seed);
            if r.value() > 18 {
                found_compound = true;
                break;
            }
        }
        assert!(
            found_compound,
            "3d6!! should sometimes exceed 18 (normal 3d6 max) due to compounding"
        );
    }

    #[test]
    fn test_keep_high_notation() {
        let result = parse("4d6kh3");
        assert!(result.success(), "4d6kh3 should parse");
        let expr = result.expression().unwrap();
        let roll_result = roll_seeded(expr, 42);
        assert!(
            roll_result.value() >= 3 && roll_result.value() <= 18,
            "4d6kh3 should be in [3, 18]"
        );
    }

    #[test]
    fn test_keep_low_notation() {
        let result = parse("4d6kl1");
        assert!(result.success(), "4d6kl1 should parse");
        let expr = result.expression().unwrap();
        let roll_result = roll_seeded(expr, 42);
        assert!(
            roll_result.value() >= 1 && roll_result.value() <= 6,
            "4d6kl1 should be in [1, 6]"
        );
    }

    #[test]
    fn test_drop_high_notation() {
        let result = parse("4d6dh1");
        assert!(result.success(), "4d6dh1 should parse");
        let expr = result.expression().unwrap();
        let roll_result = roll_seeded(expr, 42);
        assert!(
            roll_result.value() >= 3 && roll_result.value() <= 18,
            "4d6dh1 should be in [3, 18]"
        );
    }

    #[test]
    fn test_target_notation() {
        let result = parse("4d6t4");
        assert!(result.success(), "4d6t4 should parse");
        let expr = result.expression().unwrap();
        let roll_result = roll_seeded(expr, 42);
        assert!(
            roll_result.value() >= 0 && roll_result.value() <= 4,
            "4d6t4 count should be in [0, 4]"
        );
    }

    #[test]
    fn test_min_cap_notation() {
        let result = parse("4d6mi3");
        assert!(result.success(), "4d6mi3 should parse");
        let expr = result.expression().unwrap();
        let roll_result = roll_seeded(expr, 42);
        assert!(
            roll_result.value() >= 12 && roll_result.value() <= 24,
            "4d6mi3 should be in [12, 24]"
        );
    }

    #[test]
    fn test_max_cap_notation() {
        let result = parse("4d6ma4");
        assert!(result.success(), "4d6ma4 should parse");
        let expr = result.expression().unwrap();
        let roll_result = roll_seeded(expr, 42);
        assert!(
            roll_result.value() >= 4 && roll_result.value() <= 16,
            "4d6ma4 should be in [4, 16]"
        );
    }

    #[test]
    fn test_variable_fudge_notation() {
        let result = parse("dF.2");
        assert!(result.success(), "dF.2 should parse");
        let expr = result.expression().unwrap();
        let roll_result = roll_seeded(expr, 42);
        assert!(
            roll_result.value() >= -2 && roll_result.value() <= 2,
            "dF.2 should be in [-2, 2]"
        );
    }

    #[test]
    fn test_reroll_once_notation() {
        let result = parse("2d6ro1");
        assert!(result.success(), "2d6ro1 should parse");
        let expr = result.expression().unwrap();
        let roll_result = roll_seeded(expr, 42);
        assert!(
            roll_result.value() >= 2 && roll_result.value() <= 12,
            "2d6ro1 should be in [2, 12]"
        );
    }

    #[test]
    fn test_count_success_notation() {
        let result = parse("4d6cs>=4");
        assert!(result.success(), "4d6cs>=4 should parse");
        let expr = result.expression().unwrap();
        let roll_result = roll_seeded(expr, 42);
        assert!(
            roll_result.value() >= 0 && roll_result.value() <= 4,
            "4d6cs>=4 count should be in [0, 4]"
        );
    }

    // ── Error Path Tests ──────────────────────────────────────────────

    #[test]
    fn test_parse_invalid_returns_error() {
        let result = parse("@#invalid");
        assert!(!result.success());
        assert!(!result.errors().is_empty());
    }

    #[test]
    fn test_parse_empty_returns_error() {
        let result = parse("");
        assert!(!result.success());
        assert!(!result.errors().is_empty());
    }

    #[test]
    fn test_parse_d0_returns_error() {
        let result = parse("d0");
        assert!(!result.success());
        assert!(!result.errors().is_empty());
    }

    #[test]
    fn test_parse_unterminated_dice_returns_error() {
        let result = parse("3d");
        assert!(!result.success());
        assert!(!result.errors().is_empty());
    }
}
