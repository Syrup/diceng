use crate::parser::ast::*;
use crate::stats::result::*;
use crate::types::*;
use std::collections::HashMap;

/// Compute exact probability distribution for a dice expression
pub fn compute_exact(expr: &Expression) -> Option<ProbabilitiesResult> {
    match expr {
        Expression::Literal(n) => {
            let mut result = ProbabilitiesResult::new();
            result.add(*n as i64);
            Some(result)
        }
        Expression::Dice(dice_expr) => compute_dice_exact(dice_expr),
        Expression::DiceSet { exprs, reducer } => compute_dice_set_exact(exprs, *reducer),
        Expression::BinaryOp { op, left, right } => {
            let left_dist = compute_exact(left)?;
            let right_dist = compute_exact(right)?;
            Some(compute_binary_op_exact(*op, &left_dist, &right_dist))
        }
        Expression::UnaryMinus(inner) => {
            let inner_dist = compute_exact(inner)?;
            Some(compute_unary_minus_exact(&inner_dist))
        }
    }
}

fn compute_dice_exact(expr: &DiceExpression) -> Option<ProbabilitiesResult> {
    // Get the base distribution for a single die
    let single_die_dist = get_single_die_distribution(&expr.atom)?;

    // Compute distribution for N dice (convolution)
    let count = expr.atom.count() as usize;
    let base_dist = if count == 1 {
        single_die_dist
    } else {
        convolve_n(&single_die_dist, count)?
    };

    // Apply functors
    let mut current_dist = base_dist;
    for functor in &expr.functors {
        current_dist = apply_functor_exact(&current_dist, functor, &expr.atom)?;
    }

    // Apply filters (keep/drop) using DP algorithm
    if !expr.filters.is_empty() {
        current_dist = apply_filters_dp(&expr.filters, &expr.atom)?;
    }

    // Apply count threshold
    if let Some(ref threshold) = expr.count_threshold {
        current_dist = apply_count_exact(&current_dist, threshold);
    }

    Some(current_dist)
}

fn get_single_die_distribution(atom: &DiceAtom) -> Option<ProbabilitiesResult> {
    let faces = atom.face_values();
    let mut result = ProbabilitiesResult::new();

    for face in &faces {
        result.add(*face as i64);
    }

    Some(result)
}

/// Convolve two distributions (multiply them)
/// Returns None if overflow occurs
fn convolve(a: &ProbabilitiesResult, b: &ProbabilitiesResult) -> Option<ProbabilitiesResult> {
    let mut result = ProbabilitiesResult::new();

    for (&val_a, &count_a) in &a.distribution {
        for (&val_b, &count_b) in &b.distribution {
            let sum = val_a + val_b;
            let count = count_a.checked_mul(count_b)?;
            if !result.add_quantity(sum, count) {
                return None;
            }
        }
    }

    Some(result)
}

/// Convolve a distribution with itself N times
fn convolve_n(dist: &ProbabilitiesResult, n: usize) -> Option<ProbabilitiesResult> {
    if n == 0 {
        let mut result = ProbabilitiesResult::new();
        result.add(0);
        return Some(result);
    }

    if n == 1 {
        return Some(dist.clone());
    }

    let mut result = dist.clone();
    for _ in 1..n {
        result = convolve(&result, dist)?;

        // Check for memory explosion
        if result.distribution.len() > 1_000_000 {
            return None; // Too many combinations, fall back to Monte Carlo
        }
    }

    Some(result)
}

fn compute_dice_set_exact(exprs: &[Expression], reducer: Reducer) -> Option<ProbabilitiesResult> {
    // Handle empty expression set
    if exprs.is_empty() {
        let mut result = ProbabilitiesResult::new();
        result.add(0);
        return Some(result);
    }

    // Compute distribution for each expression
    let dists: Vec<ProbabilitiesResult> = exprs
        .iter()
        .map(compute_exact)
        .collect::<Option<Vec<_>>>()?;

    // Combine distributions based on reducer
    match reducer {
        Reducer::Sum => {
            let mut result = dists[0].clone();
            for dist in &dists[1..] {
                result = convolve(&result, dist)?;
            }
            Some(result)
        }
        Reducer::Min => Some(compute_min_max_exact(&dists, true)),
        Reducer::Max => Some(compute_min_max_exact(&dists, false)),
        Reducer::Average => {
            // Average is sum / count, but we need to handle rounding
            let sum_dist = {
                let mut result = dists[0].clone();
                for dist in &dists[1..] {
                    result = convolve(&result, dist)?;
                }
                result
            };

            let count = exprs.len() as f64;
            let mut result = ProbabilitiesResult::new();
            for (&value, &count_val) in &sum_dist.distribution {
                let avg = (value as f64 / count).round() as i64;
                result.add_quantity(avg, count_val);
            }
            Some(result)
        }
        Reducer::Median => {
            // Median is more complex - for exact computation, we'd need to enumerate
            // For now, return None to fall back to Monte Carlo
            None
        }
    }
}

fn compute_min_max_exact(dists: &[ProbabilitiesResult], is_min: bool) -> ProbabilitiesResult {
    // For min/max of independent distributions, we need to compute the CDF
    let mut result = ProbabilitiesResult::new();

    // Get all possible values
    let all_values: Vec<i64> = dists
        .iter()
        .flat_map(|d| d.distribution.keys().copied())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for &value in &all_values {
        let prob = if is_min {
            // P(min >= value) = product of P(Xi >= value)
            // P(min = value) = P(min >= value) - P(min >= value+1)
            let p_ge: f64 = dists
                .iter()
                .map(|d| d.reverse_cumulative_probability(value))
                .product();
            let p_ge_next: f64 = dists
                .iter()
                .map(|d| d.reverse_cumulative_probability(value + 1))
                .product();
            p_ge - p_ge_next
        } else {
            // P(max <= value) = product of P(Xi <= value)
            // P(max = value) = P(max <= value) - P(max <= value-1)
            let p_le: f64 = dists
                .iter()
                .map(|d| d.cumulative_probability(value))
                .product();
            let p_le_prev: f64 = dists
                .iter()
                .map(|d| d.cumulative_probability(value - 1))
                .product();
            p_le - p_le_prev
        };

        if prob > 1e-12 {
            let count = (prob * 1_000_000.0).round() as u64;
            if count > 0 {
                result.add_quantity(value, count);
            }
        }
    }

    result
}

fn compute_binary_op_exact(
    op: BinaryOp,
    left: &ProbabilitiesResult,
    right: &ProbabilitiesResult,
) -> ProbabilitiesResult {
    let mut result = ProbabilitiesResult::new();

    for (&val_l, &count_l) in &left.distribution {
        for (&val_r, &count_r) in &right.distribution {
            let value = match op {
                BinaryOp::Add => val_l + val_r,
                BinaryOp::Sub => val_l - val_r,
                BinaryOp::Mul => val_l * val_r,
                BinaryOp::Div => {
                    if val_r == 0 {
                        0 // Division by zero
                    } else {
                        val_l / val_r
                    }
                }
            };
            result.add_quantity(value, count_l * count_r);
        }
    }

    result
}

fn compute_unary_minus_exact(dist: &ProbabilitiesResult) -> ProbabilitiesResult {
    let mut result = ProbabilitiesResult::new();
    for (&value, &count) in &dist.distribution {
        result.add_quantity(-value, count);
    }
    result
}

fn apply_functor_exact(
    dist: &ProbabilitiesResult,
    functor: &Functor,
    atom: &DiceAtom,
) -> Option<ProbabilitiesResult> {
    match functor {
        Functor::Explode { limit, condition } => apply_explode_exact(dist, limit, condition, atom),
        Functor::Reroll { limit, condition } => apply_reroll_exact(dist, limit, condition, atom),
        Functor::Compound { limit, condition } => {
            apply_compound_exact(dist, limit, condition, atom)
        }
        Functor::Emphasis { .. } => {
            // Emphasis requires special handling - for now, fall back to Monte Carlo
            None
        }
        Functor::MinCap { min_value } => apply_min_cap_exact(dist, *min_value),
        Functor::MaxCap { max_value } => apply_max_cap_exact(dist, *max_value),
    }
}

fn apply_explode_exact(
    dist: &ProbabilitiesResult,
    limit: &FunctorLimit,
    condition: &TriggerCondition,
    atom: &DiceAtom,
) -> Option<ProbabilitiesResult> {
    let max_iterations = limit.max_count();
    let faces = atom.face_values();
    let face_count = faces.len() as f64;

    let mut result = ProbabilitiesResult::new();

    for (&value, &count) in &dist.distribution {
        let trigger_prob = get_trigger_probability(value as u32, condition, atom);

        if trigger_prob == 0.0 {
            // No explosion possible
            result.add_quantity(value, count);
        } else {
            // Compute distribution with explosions
            let mut explosion_dist = ProbabilitiesResult::new();
            explosion_dist.add(value);

            for _ in 0..max_iterations {
                let mut new_dist = ProbabilitiesResult::new();

                for (&exp_val, &exp_count) in &explosion_dist.distribution {
                    // Non-trigger case: keep current value
                    let non_trigger_prob = 1.0 - trigger_prob;
                    if non_trigger_prob > 0.0 {
                        let add_count = (exp_count as f64 * non_trigger_prob).round() as u64;
                        if add_count > 0 {
                            new_dist.add_quantity(exp_val, add_count);
                        }
                    }

                    // Trigger case: add a new die roll
                    for face in &faces {
                        let prob = trigger_prob / face_count;
                        let add_count = (exp_count as f64 * prob).round() as u64;
                        if add_count > 0 {
                            new_dist.add_quantity(exp_val + *face as i64, add_count);
                        }
                    }
                }

                explosion_dist = new_dist;
            }

            // Scale by the original count
            for (&val, &cnt) in &explosion_dist.distribution {
                result.add_quantity(val, cnt * count);
            }
        }
    }

    Some(result)
}

fn apply_reroll_exact(
    dist: &ProbabilitiesResult,
    limit: &FunctorLimit,
    condition: &TriggerCondition,
    atom: &DiceAtom,
) -> Option<ProbabilitiesResult> {
    let max_iterations = limit.max_count();
    let faces = atom.face_values();
    let face_count = faces.len() as f64;

    let mut result = ProbabilitiesResult::new();

    for (&value, &count) in &dist.distribution {
        let trigger_prob = get_trigger_probability(value as u32, condition, atom);

        if trigger_prob == 0.0 {
            // No reroll needed
            result.add_quantity(value, count);
        } else {
            // Compute distribution with rerolls
            let mut reroll_dist = ProbabilitiesResult::new();

            // First, add the non-trigger probability
            let non_trigger_prob = 1.0 - trigger_prob;
            if non_trigger_prob > 0.0 {
                let add_count = (count as f64 * non_trigger_prob).round() as u64;
                if add_count > 0 {
                    reroll_dist.add_quantity(value, add_count);
                }
            }

            // Then, compute reroll chain
            let mut current_prob = trigger_prob;
            for _ in 0..max_iterations {
                for face in &faces {
                    let face_prob = current_prob / face_count;
                    let add_count = (count as f64 * face_prob).round() as u64;
                    if add_count > 0 {
                        reroll_dist.add_quantity(*face as i64, add_count);
                    }
                }
                current_prob *= trigger_prob;
            }

            for (&val, &cnt) in &reroll_dist.distribution {
                result.add_quantity(val, cnt);
            }
        }
    }

    Some(result)
}

fn apply_compound_exact(
    dist: &ProbabilitiesResult,
    limit: &FunctorLimit,
    condition: &TriggerCondition,
    atom: &DiceAtom,
) -> Option<ProbabilitiesResult> {
    let max_iterations = limit.max_count();
    let faces = atom.face_values();
    let face_count = faces.len() as f64;

    let mut result = ProbabilitiesResult::new();

    for (&value, &count) in &dist.distribution {
        let trigger_prob = get_trigger_probability(value as u32, condition, atom);

        if trigger_prob == 0.0 {
            // No compound possible
            result.add_quantity(value, count);
        } else {
            // Compute distribution with compound
            let mut compound_dist = ProbabilitiesResult::new();
            compound_dist.add(value);

            for _ in 0..max_iterations {
                let mut new_dist = ProbabilitiesResult::new();

                for (&comp_val, &comp_count) in &compound_dist.distribution {
                    // Non-trigger case
                    let non_trigger_prob = 1.0 - trigger_prob;
                    if non_trigger_prob > 0.0 {
                        let add_count = (comp_count as f64 * non_trigger_prob).round() as u64;
                        if add_count > 0 {
                            new_dist.add_quantity(comp_val, add_count);
                        }
                    }

                    // Trigger case: add to existing value
                    for face in &faces {
                        let prob = trigger_prob / face_count;
                        let add_count = (comp_count as f64 * prob).round() as u64;
                        if add_count > 0 {
                            new_dist.add_quantity(comp_val + *face as i64, add_count);
                        }
                    }
                }

                compound_dist = new_dist;
            }

            // Scale by original count
            for (&val, &cnt) in &compound_dist.distribution {
                result.add_quantity(val, cnt * count);
            }
        }
    }

    Some(result)
}

fn get_trigger_probability(value: u32, condition: &TriggerCondition, atom: &DiceAtom) -> f64 {
    let faces = atom.face_values();

    match condition {
        TriggerCondition::Exact(target) => {
            if value == *target {
                1.0
            } else {
                0.0
            }
        }
        TriggerCondition::AtOrAbove(threshold) => {
            if value >= *threshold {
                1.0
            } else {
                0.0
            }
        }
        TriggerCondition::AtOrBelow(threshold) => {
            if value <= *threshold {
                1.0
            } else {
                0.0
            }
        }
        TriggerCondition::Between(low, high) => {
            if value >= *low && value <= *high {
                1.0
            } else {
                0.0
            }
        }
        TriggerCondition::Max => {
            let max_val = faces.iter().max().unwrap_or(&0);
            if value as i32 == *max_val {
                1.0
            } else {
                0.0
            }
        }
    }
}

/// Compute binomial coefficient C(n, k)
fn binomial(n: usize, k: usize) -> u64 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result: u64 = 1;
    for i in 0..k {
        result = result * (n - i) as u64 / (i + 1) as u64;
    }
    result
}

/// Compute exact distribution for keep/drop using Eisenstat's DP algorithm.
///
/// Instead of brute-force enumeration (O(sides^count)), this uses dynamic programming
/// with complexity O(count^3 * sides^2 * drop_highest).
///
/// The algorithm iterates over face values from highest to lowest, considering how
/// many dice show each value. For each count of dice showing the current max value,
/// it uses binomial coefficients to weight the combinations and recurses on the
/// remaining dice with remaining face values.
fn outcomes_dp(
    count: usize,
    face_values: &[i32],
    face_idx: usize,
    drop_highest: usize,
    drop_lowest: usize,
    memo: &mut HashMap<(usize, usize, usize, usize), HashMap<i64, u64>>,
) -> HashMap<i64, u64> {
    let key = (count, face_idx, drop_highest, drop_lowest);
    if let Some(cached) = memo.get(&key) {
        return cached.clone();
    }

    let mut result = HashMap::new();

    if count == 0 {
        result.insert(0i64, 1u64);
    } else if face_idx < face_values.len() {
        let current_face = face_values[face_idx] as i64;

        for showing_max in 0..=count {
            // Recurse on remaining dice with remaining face values
            let drop_highest_for_sub = drop_highest.saturating_sub(showing_max);
            let sub = outcomes_dp(
                count - showing_max,
                face_values,
                face_idx + 1,
                drop_highest_for_sub,
                drop_lowest,
                memo,
            );

            // How many of the dice showing max are actually kept
            let kept_showing_max = showing_max
                .saturating_sub(drop_highest)
                .min(count.saturating_sub(drop_highest + drop_lowest));
            let sum_from_max = kept_showing_max as i64 * current_face;

            let multiplier = binomial(count, showing_max);

            for (&sum, &ways) in &sub {
                *result.entry(sum_from_max + sum).or_insert(0) += multiplier * ways;
            }
        }
    }

    memo.insert(key, result.clone());
    result
}

/// Apply keep/drop filters using the DP algorithm.
/// This replaces the brute-force `generate_all_outcomes` approach.
fn apply_filters_dp(filters: &[Filter], atom: &DiceAtom) -> Option<ProbabilitiesResult> {
    let count = atom.count() as usize;

    // Convert filters to (drop_highest, drop_lowest) parameters
    let (mut drop_highest, mut drop_lowest) = (0usize, 0usize);

    for filter in filters {
        let n = filter.n as usize;
        match (filter.filter_type, filter.direction) {
            (FilterType::Keep, FilterDirection::Highest) => {
                // Keep highest N = drop lowest (count - N)
                drop_lowest = drop_lowest.max(count.saturating_sub(n));
            }
            (FilterType::Keep, FilterDirection::Lowest) => {
                // Keep lowest N = drop highest (count - N)
                drop_highest = drop_highest.max(count.saturating_sub(n));
            }
            (FilterType::Keep, FilterDirection::Middle) => {
                // Keep middle N = drop some from both sides
                let total_drop = count.saturating_sub(n);
                let drop_each = total_drop / 2;
                drop_highest = drop_highest.max(drop_each);
                drop_lowest = drop_lowest.max(total_drop - drop_each);
            }
            (FilterType::Drop, FilterDirection::Lowest) => {
                drop_lowest = drop_lowest.max(n);
            }
            (FilterType::Drop, FilterDirection::Highest) => {
                drop_highest = drop_highest.max(n);
            }
            (FilterType::Drop, FilterDirection::Middle) => {
                let drop_each = n / 2;
                drop_highest = drop_highest.max(drop_each);
                drop_lowest = drop_lowest.max(n - drop_each);
            }
        }
    }

    // Sanity check
    if drop_highest + drop_lowest >= count {
        return None;
    }

    // Get sorted face values (highest first) for the DP algorithm
    let mut face_values = atom.face_values();
    face_values.sort_by(|a, b| b.cmp(a)); // Sort descending

    let mut memo = HashMap::new();
    let outcomes = outcomes_dp(count, &face_values, 0, drop_highest, drop_lowest, &mut memo);

    let mut result = ProbabilitiesResult::new();
    for (&sum, &ways) in &outcomes {
        result.add_quantity(sum, ways);
    }

    Some(result)
}

fn apply_count_exact(
    dist: &ProbabilitiesResult,
    threshold: &MultiCountThreshold,
) -> ProbabilitiesResult {
    let mut result = ProbabilitiesResult::new();

    for (&value, &count) in &dist.distribution {
        let value_u32 = value as u32;
        let mut matches = 0u32;

        for t in &threshold.thresholds {
            let does_match = match t.op {
                CountOp::Eq => value_u32 == t.value,
                CountOp::Ne => value_u32 != t.value,
                CountOp::Lt => value_u32 < t.value,
                CountOp::Le => value_u32 <= t.value,
                CountOp::Gt => value_u32 > t.value,
                CountOp::Ge => value_u32 >= t.value,
            };
            if does_match {
                matches += 1;
            }
        }

        result.add_quantity(matches as i64, count);
    }

    result
}

fn apply_min_cap_exact(dist: &ProbabilitiesResult, min_value: u32) -> Option<ProbabilitiesResult> {
    let min = min_value as i64;
    let mut result = ProbabilitiesResult::new();
    for (&value, &count) in &dist.distribution {
        let capped = value.max(min);
        result.add_quantity(capped, count);
    }
    Some(result)
}

fn apply_max_cap_exact(dist: &ProbabilitiesResult, max_value: u32) -> Option<ProbabilitiesResult> {
    let max = max_value as i64;
    let mut result = ProbabilitiesResult::new();
    for (&value, &count) in &dist.distribution {
        let capped = value.min(max);
        result.add_quantity(capped, count);
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_exact_d6() {
        let expr = Parser::parse("d6").expression().unwrap().clone();
        let dist = compute_exact(&expr).unwrap();

        assert_eq!(dist.total, 6);
        for i in 1..=6 {
            assert_eq!(dist.probability(i), 1.0 / 6.0);
        }
    }

    #[test]
    fn test_exact_2d6() {
        let expr = Parser::parse("2d6").expression().unwrap().clone();
        let dist = compute_exact(&expr).unwrap();

        // 2d6 has 36 total outcomes
        assert_eq!(dist.total, 36);

        // Sum of 2 should be 1/36
        assert!((dist.probability(2) - 1.0 / 36.0).abs() < 1e-10);

        // Sum of 7 should be 6/36 = 1/6
        assert!((dist.probability(7) - 1.0 / 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_exact_literal() {
        let expr = Parser::parse("42").expression().unwrap().clone();
        let dist = compute_exact(&expr).unwrap();

        assert_eq!(dist.total, 1);
        assert_eq!(dist.probability(42), 1.0);
    }

    #[test]
    fn test_exact_arithmetic() {
        let expr = Parser::parse("d6+2").expression().unwrap().clone();
        let dist = compute_exact(&expr).unwrap();

        // Should be uniform from 3 to 8
        for i in 3..=8 {
            assert!((dist.probability(i) - 1.0 / 6.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_4d6_keep_3() {
        let expr = Parser::parse("4d6 keep 3").expression().unwrap().clone();
        let dist = compute_exact(&expr).unwrap();
        let stats = dist.stats();

        // 4d6k3: keep highest 3 of 4 dice
        assert_eq!(stats.min, 3); // 3 dice × 1
        assert_eq!(stats.max, 18); // 3 dice × 6
        assert!(stats.mean > 12.0 && stats.mean < 13.0);
    }

    #[test]
    fn test_10d6_keep_3() {
        let expr = Parser::parse("10d6 keep 3").expression().unwrap().clone();
        let dist = compute_exact(&expr).unwrap();
        let stats = dist.stats();

        // 10d6k3: keep highest 3 of 10 dice
        assert_eq!(stats.min, 3); // 3 dice × 1
        assert_eq!(stats.max, 18); // 3 dice × 6
                                   // Mean should be around 16 (very likely to get high values with 10 dice)
        assert!(stats.mean > 15.5 && stats.mean < 16.5);
    }

    #[test]
    fn test_15d6_keep_1() {
        let expr = Parser::parse("15d6 keep 1").expression().unwrap().clone();
        let dist = compute_exact(&expr).unwrap();
        let stats = dist.stats();

        // 15d6k1: keep highest 1 of 15 dice
        assert_eq!(stats.min, 1);
        assert_eq!(stats.max, 6);
        // Mean should be close to 6 (very high chance of getting at least one 6)
        assert!(stats.mean > 5.5 && stats.mean < 6.0);
    }

    #[test]
    fn test_20d6_keep_5() {
        let expr = Parser::parse("20d6 keep 5").expression().unwrap().clone();
        let dist = compute_exact(&expr).unwrap();
        let stats = dist.stats();

        // 20d6k5: keep highest 5 of 20 dice
        assert_eq!(stats.min, 5); // 5 dice × 1
        assert_eq!(stats.max, 30); // 5 dice × 6
        assert!(stats.mean > 27.0 && stats.mean < 29.0);
    }

    #[test]
    fn test_4d6_drop_1() {
        let expr = Parser::parse("4d6 drop 1").expression().unwrap().clone();
        let dist = compute_exact(&expr).unwrap();
        let stats = dist.stats();

        // 4d6d1 = 4d6k3 (same thing)
        assert_eq!(stats.min, 3);
        assert_eq!(stats.max, 18);
    }

    #[test]
    fn test_5d6_keep_middle_3() {
        let expr = Parser::parse("5d6 keep middle 3")
            .expression()
            .unwrap()
            .clone();
        let dist = compute_exact(&expr).unwrap();
        let stats = dist.stats();

        // 5d6 keep middle 3: drop highest 1 and lowest 1
        assert_eq!(stats.min, 3);
        assert_eq!(stats.max, 18);
    }

    #[test]
    fn test_dp_total_matches_bruteforce() {
        // Verify DP produces same total as convolution for simple case
        let expr = Parser::parse("4d6 keep 3").expression().unwrap().clone();
        let dist = compute_exact(&expr).unwrap();

        // Total should be 6^4 = 1296 (all possible outcomes of 4d6)
        assert_eq!(dist.total, 1296);
    }

    // ── Coverage Gap Tests ────────────────────────────────────────────

    #[test]
    fn test_exact_fate_dice() {
        let expr = Parser::parse("dF").expression().unwrap().clone();
        let dist = compute_exact(&expr).unwrap();

        // dF has 3 faces: -1, 0, 1
        assert_eq!(dist.total, 3);
        assert!((dist.probability(-1) - 1.0 / 3.0).abs() < 1e-10);
        assert!((dist.probability(0) - 1.0 / 3.0).abs() < 1e-10);
        assert!((dist.probability(1) - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_exact_percent() {
        let expr = Parser::parse("d%").expression().unwrap().clone();
        let dist = compute_exact(&expr).unwrap();

        // d% has 100 faces: 1-100
        assert_eq!(dist.total, 100);
        for i in 1..=100 {
            assert!((dist.probability(i) - 1.0 / 100.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_exact_returns_none_for_emphasis() {
        let expr = Parser::parse("d6 emphasis").expression().unwrap().clone();
        let dist = compute_exact(&expr);
        // Emphasis is unsupported for exact computation
        assert!(dist.is_none());
    }

    #[test]
    fn test_exact_unary_minus() {
        let expr = Parser::parse("-d6").expression().unwrap().clone();
        let dist = compute_exact(&expr).unwrap();

        // -d6: values -6 to -1, each with probability 1/6
        assert_eq!(dist.total, 6);
        for i in -6..=-1 {
            assert!((dist.probability(i) - 1.0 / 6.0).abs() < 1e-10);
        }
    }
}
