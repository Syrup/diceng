use serde::{Deserialize, Serialize};
use std::fmt;

/// Trigger condition for explode/reroll/compound
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerCondition {
    /// Exact value match (e.g., "on 6")
    Exact(u32),
    /// At or above threshold (e.g., "on 5 or more")
    AtOrAbove(u32),
    /// At or below threshold (e.g., "on 2 or less")
    AtOrBelow(u32),
    /// Between two values inclusive (e.g., "on 3..5")
    Between(u32, u32),
    /// On the die's maximum face value
    Max,
}

/// Limit on how many times a functor can fire
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctorLimit {
    /// No limit (default)
    Always,
    /// At most once
    Once,
    /// At most twice
    Twice,
    /// At most three times
    Thrice,
    /// At most N times
    Times(u32),
}

impl FunctorLimit {
    pub fn max_count(&self) -> u32 {
        match self {
            FunctorLimit::Always => 100, // safety cap
            FunctorLimit::Once => 1,
            FunctorLimit::Twice => 2,
            FunctorLimit::Thrice => 3,
            FunctorLimit::Times(n) => *n,
        }
    }
}

/// Direction for keep/drop filters
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterDirection {
    Highest,
    Lowest,
    Middle,
}

/// Type of filter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterType {
    Keep,
    Drop,
}

/// Reducer for expression sets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reducer {
    Sum,
    Min,
    Max,
    Average,
    Median,
}

impl fmt::Display for Reducer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reducer::Sum => write!(f, "sum"),
            Reducer::Min => write!(f, "min"),
            Reducer::Max => write!(f, "max"),
            Reducer::Average => write!(f, "average"),
            Reducer::Median => write!(f, "median"),
        }
    }
}

/// Comparison operator for dice pools (count)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CountOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl fmt::Display for CountOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CountOp::Eq => write!(f, "=="),
            CountOp::Ne => write!(f, "!="),
            CountOp::Lt => write!(f, "<"),
            CountOp::Le => write!(f, "<="),
            CountOp::Gt => write!(f, ">"),
            CountOp::Ge => write!(f, ">="),
        }
    }
}

/// Binary arithmetic operator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
        }
    }
}

/// Tie-break strategy for emphasis rolls
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmphasisTieBreak {
    Reroll,
    High,
    Low,
}

/// Sort order for dice pool display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// A count threshold for dice pools
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountThreshold {
    pub op: CountOp,
    pub value: u32,
}

/// Multiple count thresholds chained with "and"
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiCountThreshold {
    pub thresholds: Vec<CountThreshold>,
}
