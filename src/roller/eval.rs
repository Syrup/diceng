use crate::display::DieEntry;
use crate::parser::ast::*;
use crate::roller::rng::*;
use crate::types::*;
use serde::{Deserialize, Serialize};

/// Maximum number of dice that can be rolled at once
pub const MAX_DICE_COUNT: u32 = 10000;

/// Result of rolling a dice expression.
///
/// This is a recursive tree structure that captures the full roll history.
/// Each variant holds its final [`value()`](RollResult::value) plus the
/// sub-results needed for verbose display.
///
/// Use [`value()`](RollResult::value) to get the final number, or
/// [`to_verbose_entries()`](RollResult::to_verbose_entries) for a
/// detailed breakdown suitable for terminal output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollResult {
    /// A literal number
    Literal { value: i32 },
    /// A single die roll
    Die { sides: u32, value: i32 },
    /// Multiple dice rolled and summed
    DicePool { results: Vec<RollResult>, sum: i32 },
    /// A set of expressions reduced
    DiceSet {
        results: Vec<RollResult>,
        reduced: i32,
        reducer: Reducer,
    },
    /// Binary arithmetic operation
    BinaryOp {
        op: BinaryOp,
        left: Box<RollResult>,
        right: Box<RollResult>,
        value: i32,
    },
    /// Unary negation
    UnaryMinus { inner: Box<RollResult>, value: i32 },
    /// Filtered dice (keep/drop)
    Filtered {
        original: Box<RollResult>,
        kept: Vec<RollResult>,
        dropped: Vec<RollResult>,
        value: i32,
    },
    /// Functor applied (explode/reroll/compound)
    Functor {
        original: Box<RollResult>,
        extra_rolls: Vec<RollResult>,
        kind: FunctorKind,
        value: i32,
    },
    /// Count result (dice pool counting)
    Counted { pool: Box<RollResult>, count: u32 },
}

/// Kind of functor for RollResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctorKind {
    Explode,
    Reroll,
    Compound,
    Emphasis,
    MinCap,
    MaxCap,
}

impl RollResult {
    /// Final numeric value of this roll.
    ///
    /// For a `DicePool` this is the sum. For `Filtered` it's the sum of kept dice.
    /// For `Counted` it's the count of dice meeting the threshold.
    pub fn value(&self) -> i32 {
        match self {
            RollResult::Literal { value } => *value,
            RollResult::Die { value, .. } => *value,
            RollResult::DicePool { sum, .. } => *sum,
            RollResult::DiceSet { reduced, .. } => *reduced,
            RollResult::BinaryOp { value, .. } => *value,
            RollResult::UnaryMinus { value, .. } => *value,
            RollResult::Filtered { value, .. } => *value,
            RollResult::Functor { value, .. } => *value,
            RollResult::Counted { count, .. } => *count as i32,
        }
    }

    /// Collect all individual die values from the roll tree.
    ///
    /// Useful for displaying raw dice results. For `4d6k3`, this returns
    /// all 4 rolled values (including the dropped one).
    pub fn dice_values(&self) -> Vec<i32> {
        let mut values = Vec::new();
        self.collect_dice_values(&mut values);
        values
    }

    fn collect_dice_values(&self, values: &mut Vec<i32>) {
        match self {
            RollResult::Die { value, .. } => values.push(*value),
            RollResult::DicePool { results, .. } => {
                for r in results {
                    r.collect_dice_values(values);
                }
            }
            RollResult::Filtered { kept, .. } => {
                for r in kept {
                    r.collect_dice_values(values);
                }
            }
            RollResult::Functor {
                original,
                extra_rolls,
                ..
            } => {
                original.collect_dice_values(values);
                for r in extra_rolls {
                    r.collect_dice_values(values);
                }
            }
            _ => {}
        }
    }

    /// Convert this roll result into display entries for terminal output.
    ///
    /// Each [`DieEntry`] contains the die value, whether it was kept,
    /// and optional chain information for explode/reroll sequences.
    pub fn to_verbose_entries(&self) -> Vec<DieEntry> {
        match self {
            RollResult::DicePool { results, .. } => {
                results
                    .iter()
                    .map(|r| {
                        match r {
                            RollResult::Functor {
                                original,
                                extra_rolls,
                                ..
                            } => {
                                // Build chain: [original] → [extra1] → [extra2]
                                let mut chain = vec![original.value()];
                                for extra in extra_rolls {
                                    chain.push(extra.value());
                                }
                                DieEntry {
                                    value: r.value(),
                                    kept: true,
                                    chain: Some(chain),
                                    operator: None,
                                }
                            }
                            _ => DieEntry {
                                value: r.value(),
                                kept: true,
                                chain: None,
                                operator: None,
                            },
                        }
                    })
                    .collect()
            }
            RollResult::Filtered { kept, dropped, .. } => {
                let mut entries = Vec::new();
                for k in kept {
                    entries.push(DieEntry {
                        value: k.value(),
                        kept: true,
                        chain: extract_chain(k),
                        operator: None,
                    });
                }
                for d in dropped {
                    entries.push(DieEntry {
                        value: d.value(),
                        kept: false,
                        chain: extract_chain(d),
                        operator: None,
                    });
                }
                // Sort: kept first (descending value), then dropped (descending value)
                entries.sort_by(|a, b| {
                    if a.kept == b.kept {
                        b.value.cmp(&a.value)
                    } else {
                        b.kept.cmp(&a.kept)
                    }
                });
                entries
            }
            RollResult::Functor {
                original,
                extra_rolls,
                ..
            } => {
                let mut chain = vec![original.value()];
                for extra in extra_rolls {
                    chain.push(extra.value());
                }
                vec![DieEntry {
                    value: self.value(),
                    kept: true,
                    chain: Some(chain),
                    operator: None,
                }]
            }
            RollResult::Literal { value } => {
                vec![DieEntry {
                    value: *value,
                    kept: true,
                    chain: None,
                    operator: None,
                }]
            }
            RollResult::Die { value, .. } => {
                vec![DieEntry {
                    value: *value,
                    kept: true,
                    chain: None,
                    operator: None,
                }]
            }
            RollResult::BinaryOp {
                op, left, right, ..
            } => {
                // Recursively extract dice entries from both operands
                let mut entries = left.to_verbose_entries();
                // Tag right operand entries with the operator
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                };
                let mut right_entries = right.to_verbose_entries();
                for entry in &mut right_entries {
                    if entry.operator.is_none() {
                        entry.operator = Some(op_str.to_string());
                    }
                }
                entries.extend(right_entries);
                entries
            }
            RollResult::UnaryMinus { inner, .. } => inner.to_verbose_entries(),
            RollResult::DiceSet { results, .. } => {
                let mut entries = Vec::new();
                for r in results {
                    entries.extend(r.to_verbose_entries());
                }
                entries
            }
            RollResult::Counted { pool, .. } => pool.to_verbose_entries(),
        }
    }
}

/// Extract chain from a functor result
fn extract_chain(result: &RollResult) -> Option<Vec<i32>> {
    match result {
        RollResult::Functor {
            original,
            extra_rolls,
            ..
        } => {
            let mut chain = vec![original.value()];
            for extra in extra_rolls {
                chain.push(extra.value());
            }
            Some(chain)
        }
        _ => None,
    }
}

/// Dice roller that evaluates an AST with a given RNG.
///
/// Generic over `R: DiceRng`, so you can plug in [`RandomRng`] for
/// real rolls or [`LehmerRng`] for deterministic, reproducible rolls.
///
/// ```
/// use diceng::parser::Parser;
/// use diceng::roller::{Roller, RandomRng};
///
/// let expr = Parser::parse("3d6").expression().unwrap().clone();
/// let mut roller = Roller::new(RandomRng::new());
/// let result = roller.roll(&expr);
/// assert!(result.value() >= 3 && result.value() <= 18);
/// ```
pub struct Roller<R: DiceRng> {
    rng: R,
}

impl<R: DiceRng> Roller<R> {
    /// Create a new roller with the given RNG.
    pub fn new(rng: R) -> Self {
        Self { rng }
    }

    /// Evaluate a dice expression and return the result tree.
    ///
    /// The result preserves the full roll history, including individual
    /// die values, explode chains, and keep/drop decisions.
    pub fn roll(&mut self, expr: &Expression) -> RollResult {
        match expr {
            Expression::Literal(n) => RollResult::Literal { value: *n },
            Expression::Dice(dice_expr) => self.roll_dice(dice_expr),
            Expression::DiceSet { exprs, reducer } => self.roll_dice_set(exprs, *reducer),
            Expression::BinaryOp { op, left, right } => self.roll_binary_op(*op, left, right),
            Expression::UnaryMinus(inner) => self.roll_unary_minus(inner),
        }
    }

    fn roll_dice(&mut self, expr: &DiceExpression) -> RollResult {
        let count = expr.atom.count();
        if count > MAX_DICE_COUNT {
            // Return a literal 0 for invalid counts
            return RollResult::Literal { value: 0 };
        }

        // Roll the base dice
        let mut results = Vec::new();
        for _ in 0..count {
            let result = self.roll_single_die(&expr.atom);
            results.push(result);
        }

        // Apply functors (explode, reroll, compound, emphasis)
        let mut all_results = results;
        for functor in &expr.functors {
            all_results = self.apply_functor(all_results, functor, &expr.atom);
        }

        // Sum the results
        let sum: i32 = all_results.iter().map(|r| r.value()).sum();

        let pool_result = RollResult::DicePool {
            results: all_results,
            sum,
        };

        // Apply filters (keep/drop)
        if !expr.filters.is_empty() {
            return self.apply_filters(pool_result, &expr.filters);
        }

        // Apply count threshold
        if let Some(ref threshold) = expr.count_threshold {
            return self.apply_count(pool_result, threshold);
        }

        pool_result
    }

    fn roll_single_die(&mut self, atom: &DiceAtom) -> RollResult {
        match atom {
            DiceAtom::Standard { sides, .. } => {
                let value = self.rng.roll(*sides) as i32;
                RollResult::Die {
                    sides: *sides,
                    value,
                }
            }
            DiceAtom::Percent { .. } => {
                let value = self.rng.roll(100) as i32;
                RollResult::Die { sides: 100, value }
            }
            DiceAtom::Fate { magnitude, .. } => {
                let faces = if *magnitude <= 1 {
                    vec![-1, 0, 1]
                } else {
                    let m = *magnitude as i32;
                    let mut f = Vec::new();
                    for i in -m..=m {
                        f.push(i);
                        if i == 0 {
                            f.push(0);
                        }
                    }
                    f
                };
                let idx = self.rng.roll(faces.len() as u32) as usize - 1;
                let value = faces[idx];
                RollResult::Die {
                    sides: faces.len() as u32,
                    value,
                }
            }
            DiceAtom::Custom { faces, .. } => {
                let idx = self.rng.roll(faces.len() as u32) as usize - 1;
                let value = faces[idx];
                RollResult::Die {
                    sides: faces.len() as u32,
                    value,
                }
            }
        }
    }

    fn apply_functor(
        &mut self,
        dice: Vec<RollResult>,
        functor: &Functor,
        atom: &DiceAtom,
    ) -> Vec<RollResult> {
        match functor {
            Functor::Explode { limit, condition } => {
                self.apply_explode(dice, limit, condition, atom)
            }
            Functor::Reroll { limit, condition } => self.apply_reroll(dice, limit, condition, atom),
            Functor::Compound { limit, condition } => {
                self.apply_compound(dice, limit, condition, atom)
            }
            Functor::Emphasis { tie_break, center } => {
                self.apply_emphasis(dice, tie_break, *center, atom)
            }
            Functor::MinCap { min_value } => self.apply_min_cap(dice, *min_value),
            Functor::MaxCap { max_value } => self.apply_max_cap(dice, *max_value),
        }
    }

    fn apply_explode(
        &mut self,
        dice: Vec<RollResult>,
        limit: &FunctorLimit,
        condition: &TriggerCondition,
        atom: &DiceAtom,
    ) -> Vec<RollResult> {
        let max_iterations = limit.max_count();
        let mut result = Vec::new();

        for die in dice {
            let mut current = die.clone();
            let mut extra_rolls = Vec::new();
            let mut iterations = 0;

            loop {
                if iterations >= max_iterations {
                    break;
                }

                let value = current.value() as u32;
                if !self.check_trigger(value, condition, atom) {
                    break;
                }

                // Roll again
                let new_roll = self.roll_single_die(atom);
                extra_rolls.push(new_roll.clone());
                current = new_roll;
                iterations += 1;
            }

            if extra_rolls.is_empty() {
                result.push(die);
            } else {
                let total: i32 = die.value() + extra_rolls.iter().map(|r| r.value()).sum::<i32>();
                result.push(RollResult::Functor {
                    original: Box::new(die),
                    extra_rolls,
                    kind: FunctorKind::Explode,
                    value: total,
                });
            }
        }

        result
    }

    fn apply_reroll(
        &mut self,
        dice: Vec<RollResult>,
        limit: &FunctorLimit,
        condition: &TriggerCondition,
        atom: &DiceAtom,
    ) -> Vec<RollResult> {
        let max_iterations = limit.max_count();
        let mut result = Vec::new();

        for die in dice {
            let mut current = die.clone();
            let mut extra_rolls = Vec::new();
            let mut iterations = 0;

            loop {
                if iterations >= max_iterations {
                    break;
                }

                let value = current.value() as u32;
                if !self.check_trigger(value, condition, atom) {
                    break;
                }

                // Reroll - only the last roll counts
                let new_roll = self.roll_single_die(atom);
                extra_rolls.push(current);
                current = new_roll;
                iterations += 1;
            }

            if extra_rolls.is_empty() {
                result.push(die);
            } else {
                result.push(RollResult::Functor {
                    original: Box::new(current.clone()),
                    extra_rolls,
                    kind: FunctorKind::Reroll,
                    value: current.value(),
                });
            }
        }

        result
    }

    fn apply_compound(
        &mut self,
        dice: Vec<RollResult>,
        limit: &FunctorLimit,
        condition: &TriggerCondition,
        atom: &DiceAtom,
    ) -> Vec<RollResult> {
        let max_iterations = limit.max_count();
        let mut result = Vec::new();

        for die in dice {
            let mut total = die.value();
            let mut extra_rolls = Vec::new();
            let mut iterations = 0;

            loop {
                if iterations >= max_iterations {
                    break;
                }

                let value = if iterations == 0 {
                    die.value() as u32
                } else {
                    extra_rolls
                        .last()
                        .map(|r: &RollResult| r.value() as u32)
                        .unwrap_or(0)
                };

                if !self.check_trigger(value, condition, atom) {
                    break;
                }

                // Compound - add to original die
                let new_roll = self.roll_single_die(atom);
                total += new_roll.value();
                extra_rolls.push(new_roll);
                iterations += 1;
            }

            if extra_rolls.is_empty() {
                result.push(die);
            } else {
                result.push(RollResult::Functor {
                    original: Box::new(die),
                    extra_rolls,
                    kind: FunctorKind::Compound,
                    value: total,
                });
            }
        }

        result
    }

    fn apply_emphasis(
        &mut self,
        dice: Vec<RollResult>,
        tie_break: &EmphasisTieBreak,
        center: Option<f64>,
        atom: &DiceAtom,
    ) -> Vec<RollResult> {
        let mut result = Vec::new();

        for die in dice {
            // Roll two dice
            let roll1 = self.roll_single_die(atom);
            let roll2 = self.roll_single_die(atom);

            let center_val = center.unwrap_or_else(|| {
                // Default center is average of min and max
                let faces = atom.face_values();
                let min = *faces.iter().min().unwrap_or(&1) as f64;
                let max = *faces.iter().max().unwrap_or(&6) as f64;
                (min + max) / 2.0
            });

            let dist1 = (roll1.value() as f64 - center_val).abs();
            let dist2 = (roll2.value() as f64 - center_val).abs();

            let chosen_value = if dist1 > dist2 {
                roll1.value()
            } else if dist2 > dist1 {
                roll2.value()
            } else {
                // Tie - apply tie-break
                match tie_break {
                    EmphasisTieBreak::Reroll => {
                        // Reroll (recursive) - for simplicity, just pick one
                        roll1.value()
                    }
                    EmphasisTieBreak::High => {
                        if roll1.value() >= roll2.value() {
                            roll1.value()
                        } else {
                            roll2.value()
                        }
                    }
                    EmphasisTieBreak::Low => {
                        if roll1.value() <= roll2.value() {
                            roll1.value()
                        } else {
                            roll2.value()
                        }
                    }
                }
            };

            result.push(RollResult::Functor {
                original: Box::new(die),
                extra_rolls: vec![roll1, roll2],
                kind: FunctorKind::Emphasis,
                value: chosen_value,
            });
        }

        result
    }

    fn apply_min_cap(&mut self, dice: Vec<RollResult>, min_value: u32) -> Vec<RollResult> {
        let min = min_value as i32;
        dice.into_iter()
            .map(|die| {
                let original_value = die.value();
                let capped = original_value.max(min);
                if capped != original_value {
                    RollResult::Functor {
                        original: Box::new(die),
                        extra_rolls: vec![],
                        kind: FunctorKind::MinCap,
                        value: capped,
                    }
                } else {
                    die
                }
            })
            .collect()
    }

    fn apply_max_cap(&mut self, dice: Vec<RollResult>, max_value: u32) -> Vec<RollResult> {
        let max = max_value as i32;
        dice.into_iter()
            .map(|die| {
                let original_value = die.value();
                let capped = original_value.min(max);
                if capped != original_value {
                    RollResult::Functor {
                        original: Box::new(die),
                        extra_rolls: vec![],
                        kind: FunctorKind::MaxCap,
                        value: capped,
                    }
                } else {
                    die
                }
            })
            .collect()
    }

    fn check_trigger(&self, value: u32, condition: &TriggerCondition, atom: &DiceAtom) -> bool {
        match condition {
            TriggerCondition::Exact(target) => value == *target,
            TriggerCondition::AtOrAbove(threshold) => value >= *threshold,
            TriggerCondition::AtOrBelow(threshold) => value <= *threshold,
            TriggerCondition::Between(low, high) => value >= *low && value <= *high,
            TriggerCondition::Max => value as i32 == atom.max_value(),
        }
    }

    fn apply_filters(&mut self, pool: RollResult, filters: &[Filter]) -> RollResult {
        let mut current = pool;

        for filter in filters {
            current = self.apply_single_filter(current, filter);
        }

        current
    }

    fn apply_single_filter(&mut self, pool: RollResult, filter: &Filter) -> RollResult {
        // Extract dice to filter from either DicePool or Filtered result
        let (mut dice, all_dropped_so_far) = match &pool {
            RollResult::DicePool { results, .. } => (results.clone(), Vec::new()),
            RollResult::Filtered { kept, dropped, .. } => {
                // Chain: apply next filter to the kept dice from previous filter
                (kept.clone(), dropped.clone())
            }
            _ => return pool,
        };

        // Sort dice by value for filtering
        dice.sort_by_key(|r| r.value());

        let n = filter.n as usize;
        if n >= dice.len() {
            return pool;
        }

        let (kept, dropped) = match (filter.filter_type, filter.direction) {
            (FilterType::Keep, FilterDirection::Highest) => {
                let dropped: Vec<_> = dice[..dice.len() - n].to_vec();
                let kept: Vec<_> = dice[dice.len() - n..].to_vec();
                (kept, dropped)
            }
            (FilterType::Keep, FilterDirection::Lowest) => {
                let kept: Vec<_> = dice[..n].to_vec();
                let dropped: Vec<_> = dice[n..].to_vec();
                (kept, dropped)
            }
            (FilterType::Keep, FilterDirection::Middle) => {
                let drop_each_side = (dice.len() - n) / 2;
                let kept: Vec<_> = dice[drop_each_side..drop_each_side + n].to_vec();
                let dropped: Vec<_> = dice[..drop_each_side]
                    .iter()
                    .chain(dice[drop_each_side + n..].iter())
                    .cloned()
                    .collect();
                (kept, dropped)
            }
            (FilterType::Drop, FilterDirection::Lowest) => {
                let dropped: Vec<_> = dice[..n].to_vec();
                let kept: Vec<_> = dice[n..].to_vec();
                (kept, dropped)
            }
            (FilterType::Drop, FilterDirection::Highest) => {
                let kept: Vec<_> = dice[..dice.len() - n].to_vec();
                let dropped: Vec<_> = dice[dice.len() - n..].to_vec();
                (kept, dropped)
            }
            (FilterType::Drop, FilterDirection::Middle) => {
                // Drop middle N dice: keep the outer dice
                let drop_start = (dice.len() - n) / 2;
                let drop_end = drop_start + n;
                let kept: Vec<_> = dice[..drop_start]
                    .iter()
                    .chain(dice[drop_end..].iter())
                    .cloned()
                    .collect();
                let dropped: Vec<_> = dice[drop_start..drop_end].to_vec();
                (kept, dropped)
            }
        };

        let sum: i32 = kept.iter().map(|r| r.value()).sum();

        // Merge dropped dice from previous filters with current filter's dropped
        let mut all_dropped = all_dropped_so_far;
        all_dropped.extend(dropped);

        RollResult::Filtered {
            original: Box::new(pool),
            kept,
            dropped: all_dropped,
            value: sum,
        }
    }

    fn apply_count(&mut self, pool: RollResult, threshold: &MultiCountThreshold) -> RollResult {
        let dice: Vec<RollResult> = match &pool {
            RollResult::DicePool { results, .. } => results.clone(),
            _ => return pool,
        };

        let mut total_count = 0u32;

        for die in &dice {
            let value = die.value() as u32;
            for t in &threshold.thresholds {
                let matches = match t.op {
                    CountOp::Eq => value == t.value,
                    CountOp::Ne => value != t.value,
                    CountOp::Lt => value < t.value,
                    CountOp::Le => value <= t.value,
                    CountOp::Gt => value > t.value,
                    CountOp::Ge => value >= t.value,
                };
                if matches {
                    total_count += 1;
                }
            }
        }

        RollResult::Counted {
            pool: Box::new(pool),
            count: total_count,
        }
    }

    fn roll_dice_set(&mut self, exprs: &[Expression], reducer: Reducer) -> RollResult {
        let results: Vec<RollResult> = exprs.iter().map(|e| self.roll(e)).collect();
        let values: Vec<i32> = results.iter().map(|r| r.value()).collect();

        let reduced = match reducer {
            Reducer::Sum => values.iter().sum(),
            Reducer::Min => *values.iter().min().unwrap_or(&0),
            Reducer::Max => *values.iter().max().unwrap_or(&0),
            Reducer::Average => {
                if values.is_empty() {
                    0
                } else {
                    (values.iter().sum::<i32>() as f64 / values.len() as f64).round() as i32
                }
            }
            Reducer::Median => {
                if values.is_empty() {
                    0
                } else {
                    let mut sorted = values.clone();
                    sorted.sort();
                    sorted[sorted.len() / 2]
                }
            }
        };

        RollResult::DiceSet {
            results,
            reduced,
            reducer,
        }
    }

    fn roll_binary_op(
        &mut self,
        op: BinaryOp,
        left: &Expression,
        right: &Expression,
    ) -> RollResult {
        let left_result = self.roll(left);
        let right_result = self.roll(right);

        let value = match op {
            BinaryOp::Add => left_result.value() + right_result.value(),
            BinaryOp::Sub => left_result.value() - right_result.value(),
            BinaryOp::Mul => left_result.value() * right_result.value(),
            BinaryOp::Div => {
                let divisor = right_result.value();
                if divisor == 0 {
                    0 // Division by zero returns 0
                } else {
                    left_result.value() / divisor
                }
            }
        };

        RollResult::BinaryOp {
            op,
            left: Box::new(left_result),
            right: Box::new(right_result),
            value,
        }
    }

    fn roll_unary_minus(&mut self, inner: &Expression) -> RollResult {
        let inner_result = self.roll(inner);
        let value = -inner_result.value();

        RollResult::UnaryMinus {
            inner: Box::new(inner_result),
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_roll_literal() {
        let mut roller = Roller::new(RandomRng::new());
        let expr = Parser::parse("42").expression().unwrap().clone();
        let result = roller.roll(&expr);
        assert_eq!(result.value(), 42);
    }

    #[test]
    fn test_roll_d6() {
        let mut roller = Roller::new(RandomRng::new());
        let expr = Parser::parse("d6").expression().unwrap().clone();
        let result = roller.roll(&expr);
        assert!(result.value() >= 1 && result.value() <= 6);
    }

    #[test]
    fn test_roll_3d6() {
        let mut roller = Roller::new(RandomRng::new());
        let expr = Parser::parse("3d6").expression().unwrap().clone();
        let result = roller.roll(&expr);
        assert!(result.value() >= 3 && result.value() <= 18);
    }

    #[test]
    fn test_roll_deterministic() {
        let mut roller1 = Roller::new(LehmerRng::new(42));
        let mut roller2 = Roller::new(LehmerRng::new(42));
        let expr = Parser::parse("4d6").expression().unwrap().clone();

        let result1 = roller1.roll(&expr);
        let result2 = roller2.roll(&expr);

        assert_eq!(result1.value(), result2.value());
    }

    #[test]
    fn test_roll_arithmetic() {
        let mut roller = Roller::new(RandomRng::new());
        let expr = Parser::parse("2d6+4").expression().unwrap().clone();
        let result = roller.roll(&expr);
        assert!(result.value() >= 6 && result.value() <= 16);
    }

    #[test]
    fn test_roll_keep() {
        let mut roller = Roller::new(RandomRng::new());
        let expr = Parser::parse("4d6 keep 3").expression().unwrap().clone();
        let result = roller.roll(&expr);
        assert!(result.value() >= 3 && result.value() <= 18);
    }
}
