use diceng::{
    compute_distribution, exact_distribution, monte_carlo_distribution, parse, roll_seeded,
};

// ── Full Pipeline Tests ──────────────────────────────────────────────

#[test]
fn test_full_pipeline_parse_roll_display() {
    let result = parse("4d6kh3");
    assert!(result.success(), "4d6kh3 should parse");

    let expr = result.expression().unwrap();
    let roll_result = roll_seeded(expr, 42);
    let value = roll_result.value();

    assert!(
        value >= 3 && value <= 18,
        "4d6kh3 should be in [3, 18], got {}",
        value
    );

    let entries = roll_result.to_verbose_entries();
    assert_eq!(
        entries.len(),
        4,
        "4d6kh3 should have 4 entries (3 kept + 1 dropped)"
    );

    let kept_count = entries.iter().filter(|e| e.kept).count();
    assert_eq!(kept_count, 3, "Should have 3 kept dice");
}

#[test]
fn test_full_pipeline_parse_exact_stats() {
    let result = parse("2d6");
    assert!(result.success());

    let expr = result.expression().unwrap();
    let dist = exact_distribution(expr).unwrap();

    assert_eq!(dist.total, 36);

    let stats = dist.stats();
    assert_eq!(stats.min, 2);
    assert_eq!(stats.max, 12);
    assert!((stats.mean - 7.0).abs() < 1e-10);
}

#[test]
fn test_public_api_error_handling() {
    // Invalid input
    assert!(!parse("@#invalid").success());
    assert!(!parse("").success());
    assert!(!parse("d0").success());
    assert!(!parse("3d").success());

    // Valid input should succeed
    assert!(parse("3d6").success());
    assert!(parse("4d6k3").success());
    assert!(parse("d20").success());
}

#[test]
fn test_deterministic_across_calls() {
    let expr = parse("4d6kh3").expression().unwrap().clone();

    let r1 = roll_seeded(&expr, 12345);
    let r2 = roll_seeded(&expr, 12345);
    let r3 = roll_seeded(&expr, 12345);

    assert_eq!(r1.value(), r2.value());
    assert_eq!(r2.value(), r3.value());
}

#[test]
fn test_complex_expression_end_to_end() {
    let result = parse("20d6 keep 5");
    assert!(result.success(), "20d6 keep 5 should parse");

    let expr = result.expression().unwrap();

    // Roll
    let roll_result = roll_seeded(expr, 42);
    assert!(roll_result.value() >= 5 && roll_result.value() <= 30);

    // Exact distribution
    let dist = exact_distribution(expr).unwrap();
    let stats = dist.stats();
    assert_eq!(stats.min, 5);
    assert_eq!(stats.max, 30);
}

#[test]
fn test_dice_set_end_to_end() {
    let result = parse("(2d6, 3d6) sum");
    assert!(result.success());

    let expr = result.expression().unwrap();
    let roll_result = roll_seeded(expr, 42);
    assert!(roll_result.value() >= 5 && roll_result.value() <= 30);
}

#[test]
fn test_unary_minus_end_to_end() {
    let result = parse("-d6 + 10");
    assert!(result.success());

    let expr = result.expression().unwrap();
    for seed in 1..20 {
        let r = roll_seeded(expr, seed);
        assert!(
            r.value() >= 4 && r.value() <= 9,
            "-d6+10 should be in [4,9], got {}",
            r.value()
        );
    }
}

#[test]
fn test_compute_distribution_fallback() {
    // Exact works for simple cases
    let expr = parse("2d6").expression().unwrap().clone();
    let dist = compute_distribution(&expr, 10000);
    assert_eq!(dist.total, 36); // exact

    // Monte Carlo fallback for emphasis (unsupported by exact)
    let expr2 = parse("d6 emphasis").expression().unwrap().clone();
    let dist2 = compute_distribution(&expr2, 10000);
    assert!(dist2.total > 0); // monte carlo
}

#[test]
fn test_monte_carlo_public_api() {
    let expr = parse("d6").expression().unwrap().clone();
    let dist = monte_carlo_distribution(&expr, 10000);
    assert!(dist.total >= 9000); // close to 10000
}

#[test]
fn test_all_notation_features_parse() {
    let expressions = vec![
        "3d6!",
        "3d6!!",
        "3d6!>=5",
        "4d6kh3",
        "4d6kl1",
        "4d6dh1",
        "4d6dl1",
        "2d6ro1",
        "4d6cs>=4",
        "4d6t4",
        "4d6mi2",
        "4d6ma5",
        "4d6sa",
        "4d6sd",
        "dF",
        "dF.2",
        "d%",
        "d{1,2,3}",
        "(2d6, 3d6) sum",
        "[d6, d8] max",
        "-d6 + 10",
    ];

    for expr_str in expressions {
        let result = parse(expr_str);
        assert!(result.success(), "'{}' should parse successfully", expr_str);
    }
}

// ── Dice Pool Integration Tests ──────────────────────────────────────

#[test]
fn test_dice_pool_parse_roll() {
    let result = parse("{4d6, 3d8, 2d10}kh");
    assert!(result.success(), "dice pool should parse");

    let expr = result.expression().unwrap();
    let roll_result = roll_seeded(expr, 42);
    // kh keeps only the highest group, so result should be >= 1
    assert!(roll_result.value() >= 1);
}

#[test]
fn test_dice_pool_with_literal() {
    let result = parse("{1d20, 10}kh");
    assert!(result.success(), "pool with literal should parse");

    let expr = result.expression().unwrap();
    for seed in 1..20 {
        let r = roll_seeded(expr, seed);
        // max(d20, 10): always >= 10
        assert!(
            r.value() >= 10,
            "pool kh should be >= 10, got {}",
            r.value()
        );
    }
}

#[test]
fn test_dice_pool_exact_distribution() {
    let result = parse("{d6, d8}kh");
    assert!(result.success());

    let expr = result.expression().unwrap();
    let dist = exact_distribution(expr);
    assert!(dist.is_some(), "Exact should work for pool kh");

    let dist = dist.unwrap();
    let stats = dist.stats();
    assert_eq!(stats.min, 1);
    assert_eq!(stats.max, 8);
}

#[test]
fn test_dice_pool_deterministic() {
    let expr = parse("{4d6, 3d8}kh").expression().unwrap().clone();

    let r1 = roll_seeded(&expr, 12345);
    let r2 = roll_seeded(&expr, 12345);
    assert_eq!(r1.value(), r2.value());
}

#[test]
fn test_dice_pool_notation_variants() {
    let expressions = vec![
        "{4d6, 3d8, 2d10}kh",
        "{4d6, 3d8, 2d10}kl",
        "{4d6, 3d8, 2d10}kh2",
        "{4d6, 3d8, 2d10}dh1",
        "{4d6, 3d8, 2d10}dl1",
        "{6d6, 5d8}cs>15",
        "{4d6, 3d8}",
        "{1d20, 10}kh",
        "{4d6kh3, 4d6kh3, 4d6kh3}",
    ];

    for expr_str in expressions {
        let result = parse(expr_str);
        assert!(result.success(), "'{}' should parse successfully", expr_str);
    }
}

#[test]
fn test_dice_pool_foundry_wild_die() {
    // SWADE Wild Die: {1d8x, 1d6x}kh
    // Simplified without explode: {1d8, 1d6}kh
    let result = parse("{1d8, 1d6}kh");
    assert!(result.success(), "Wild die pattern should parse");

    let expr = result.expression().unwrap();
    for seed in 1..50 {
        let r = roll_seeded(expr, seed);
        // max(d8, d6): range [1, 8]
        assert!(r.value() >= 1 && r.value() <= 8);
    }
}
