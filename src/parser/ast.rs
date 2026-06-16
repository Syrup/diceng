use crate::types::*;
use serde::{Deserialize, Serialize};

/// The core dice notation before modifiers are applied.
///
/// This is the "NdS" part of a dice expression. Everything else
/// (exploding, keeping, rerolling) is layered on top via
/// [`Functor`] and [`Filter`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiceAtom {
    /// Standard dice: `3d6`, `d20`
    Standard { count: u32, sides: u32 },
    /// Percent die: `d%` or `d100` (faces 1-100)
    Percent { count: u32 },
    /// Fate/Fudge die: `dF` (3 faces) or `dF.2` (6 faces, magnitude 2)
    Fate { count: u32, magnitude: u32 },
    /// Custom die with explicit face values: `d{1,1,2,3}`
    Custom { count: u32, faces: Vec<i32> },
}

impl DiceAtom {
    /// Get the possible face values for this die
    pub fn face_values(&self) -> Vec<i32> {
        match self {
            DiceAtom::Standard { sides, .. } => (1..=*sides as i32).collect(),
            DiceAtom::Percent { .. } => (1..=100).collect(),
            DiceAtom::Fate { magnitude, .. } => {
                if *magnitude <= 1 {
                    vec![-1, 0, 1]
                } else {
                    let m = *magnitude as i32;
                    let mut faces = Vec::new();
                    for i in -m..=m {
                        faces.push(i);
                        if i == 0 {
                            faces.push(0);
                        }
                    }
                    faces
                }
            }
            DiceAtom::Custom { faces, .. } => faces.clone(),
        }
    }

    /// Get the number of faces
    pub fn face_count(&self) -> usize {
        match self {
            DiceAtom::Standard { sides, .. } => *sides as usize,
            DiceAtom::Percent { .. } => 100,
            DiceAtom::Fate { magnitude, .. } => {
                if *magnitude <= 1 {
                    3
                } else {
                    (2 * *magnitude as usize) + 2
                }
            }
            DiceAtom::Custom { faces, .. } => faces.len(),
        }
    }

    /// Get the count of dice
    pub fn count(&self) -> u32 {
        match self {
            DiceAtom::Standard { count, .. } => *count,
            DiceAtom::Percent { count, .. } => *count,
            DiceAtom::Fate { count, .. } => *count,
            DiceAtom::Custom { count, .. } => *count,
        }
    }

    /// Get the maximum face value
    pub fn max_value(&self) -> i32 {
        self.face_values().into_iter().max().unwrap_or(0)
    }
}

/// A modifier applied to individual dice after they're rolled.
///
/// Functors change *how* dice behave: exploding adds extra rolls,
/// rerolling replaces bad results, capping clamps values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Functor {
    /// Roll an extra die when the trigger condition is met (e.g., `3d6!`, `3d6e5`)
    Explode {
        limit: FunctorLimit,
        condition: TriggerCondition,
    },
    /// Replace a die result when the trigger condition is met (e.g., `3d6r1`)
    Reroll {
        limit: FunctorLimit,
        condition: TriggerCondition,
    },
    /// Like explode, but adds to the same die value (e.g., `3d6!!`)
    Compound {
        limit: FunctorLimit,
        condition: TriggerCondition,
    },
    /// Roll two dice, take the one furthest from center (L5R/Genesys style)
    Emphasis {
        tie_break: EmphasisTieBreak,
        center: Option<f64>,
    },
    /// Clamp each die to at least `min_value` (e.g., `4d6mi2`)
    MinCap { min_value: u32 },
    /// Clamp each die to at most `max_value` (e.g., `4d6ma5`)
    MaxCap { max_value: u32 },
}

/// A keep/drop filter applied to a dice pool.
///
/// Filters select which dice to keep or drop after rolling.
/// For `4d6k3`, this would be `Keep` + `Highest` + `n = 3`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Filter {
    pub filter_type: FilterType,
    pub n: u32,
    pub direction: FilterDirection,
}

/// A complete dice expression: atom + modifiers.
///
/// This is what gets rolled. The atom defines the dice, functors
/// change individual die behavior, filters select which dice to keep,
/// and count_threshold counts dice meeting a condition.
///
/// For `4d6k3`, the atom is `Standard { count: 4, sides: 6 }` with
/// one filter: `Keep` 3 `Highest`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiceExpression {
    pub atom: DiceAtom,
    pub functors: Vec<Functor>,
    pub filters: Vec<Filter>,
    pub count_threshold: Option<MultiCountThreshold>,
    pub sort_order: Option<SortOrder>,
}

/// Top-level expression node in the AST.
///
/// An expression can be a literal number, a dice expression with modifiers,
/// a set of expressions combined with a reducer, or an arithmetic operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    /// A bare number: `42`
    Literal(i32),
    /// A dice expression with all its modifiers: `4d6k3`, `3d6!>=5`
    Dice(DiceExpression),
    /// Multiple expressions combined: `(2d6, 3d6) sum`, `[d6, d8] max`
    DiceSet {
        exprs: Vec<Expression>,
        reducer: Reducer,
    },
    /// Arithmetic: `3d6 + 4`, `2d6 * 3`
    BinaryOp {
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// Negation: `-d6 + 10`
    UnaryMinus(Box<Expression>),
}
