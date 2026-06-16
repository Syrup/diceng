use serde::{Deserialize, Serialize};
use std::fmt;

/// A parse error with the position in the input string where it occurred.
///
/// The `message` describes what went wrong. The `position` is the byte
/// offset into the input. If the parser has a suggestion for a fix,
/// it goes in `suggestion`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
    pub suggestion: Option<String>,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parse error at position {}: {}",
            self.position, self.message
        )
    }
}

impl std::error::Error for ParseError {}

/// Outcome of parsing a dice expression.
///
/// Either the expression parsed successfully, or the parser collected
/// one or more errors. Use [`success()`](ParseResult::success) to check,
/// then [`expression()`](ParseResult::expression) or
/// [`errors()`](ParseResult::errors) to get the result.
///
/// ```
/// use diceng::parse;
///
/// let ok = parse("4d6k3");
/// assert!(ok.success());
///
/// let bad = parse("4d6k");
/// assert!(!bad.success());
/// assert!(!bad.errors().is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParseResult {
    Success(crate::parser::ast::Expression),
    Failure(Vec<ParseError>),
}

impl ParseResult {
    /// Returns true if parsing succeeded
    pub fn success(&self) -> bool {
        matches!(self, ParseResult::Success(_))
    }

    /// Get the parsed expression, or None if parsing failed
    pub fn expression(&self) -> Option<&crate::parser::ast::Expression> {
        match self {
            ParseResult::Success(expr) => Some(expr),
            _ => None,
        }
    }

    /// Get the list of parse errors (empty if successful)
    pub fn errors(&self) -> &[ParseError] {
        match self {
            ParseResult::Success(_) => &[],
            ParseResult::Failure(errors) => errors,
        }
    }
}
