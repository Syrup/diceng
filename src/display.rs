use colored::*;
use terminal_size::{terminal_size, Width};

/// Kind of die entry for verbose display
#[derive(Debug, Clone, PartialEq)]
pub enum DieEntryKind {
    /// Explode: `!` - extra roll added to sum
    Explode,
    /// Compound: `!!` - extra roll added to same die value
    Compound,
    /// Reroll: `r` - discarded and rerolled
    Reroll,
    /// Reroll once: `ro` - rerolled exactly once
    RerollOnce,
    /// Min cap: `mi` - value clamped to minimum
    MinCap,
    /// Max cap: `ma` - value clamped to maximum
    MaxCap,
    /// Emphasis: two rolls, pick furthest from center
    Emphasis,
    /// Counted: threshold match
    Counted,
    /// Separator: arithmetic operator between groups
    Separator,
    /// Literal: plain number (not a dice roll)
    Literal,
    /// Pool open: start of dice pool group
    PoolOpen,
    /// Pool close: end of dice pool group
    PoolClose,
    /// Pipe: separator between pool groups
    Pipe,
    /// Group open: start of parenthesized sub-expression
    GroupOpen,
    /// Group close: end of parenthesized sub-expression
    GroupClose,
}

/// Entry for a single die in verbose display
#[derive(Debug, Clone)]
pub struct DieEntry {
    pub value: i32,
    pub kept: bool,
    pub chain: Option<Vec<i32>>,
    pub operator: Option<String>, // e.g., "+4", "-1", "*2"
    pub kind: Option<DieEntryKind>,
}

/// Check if a DieEntry represents an actual dice/value (not a structural marker)
fn is_dice_entry(die: &DieEntry) -> bool {
    !matches!(
        die.kind,
        Some(
            DieEntryKind::Separator
                | DieEntryKind::GroupOpen
                | DieEntryKind::GroupClose
                | DieEntryKind::PoolOpen
                | DieEntryKind::PoolClose
                | DieEntryKind::Pipe
        )
    )
}

/// Build expression display string from entries.
/// Returns `Some("(5 + 1) * 2")` when groups exist, `None` for additive-only.
fn compute_expression_display(dice: &[DieEntry]) -> Option<String> {
    if !dice.iter().any(|d| d.kind == Some(DieEntryKind::GroupOpen)) {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    let mut last_was_value = false;
    for die in dice {
        match &die.kind {
            Some(DieEntryKind::GroupOpen) => {
                parts.push("(".to_string());
                last_was_value = false;
            }
            Some(DieEntryKind::GroupClose) => {
                parts.push(")".to_string());
                last_was_value = true;
            }
            Some(DieEntryKind::Separator) => {
                if let Some(ref op) = die.operator {
                    parts.push(format!(" {} ", op));
                }
                last_was_value = false;
            }
            _ => {
                if last_was_value {
                    parts.push(" + ".to_string());
                }
                parts.push(die.value.to_string());
                last_was_value = true;
            }
        }
    }
    Some(parts.join(""))
}

/// Render verbose roll output in borderless spaced layout
pub fn render_verbose(expression: &str, result: i32, dice: &[DieEntry]) -> String {
    let width = terminal_width();
    let mut out = String::new();

    // Expression line
    out.push_str(&format!(
        "{} = {}",
        expression.bold(),
        result.to_string().bold()
    ));
    out.push('\n');
    out.push('\n');

    // Check if this is a count result
    let is_counted = dice.iter().any(|d| d.kind == Some(DieEntryKind::Counted));

    // Dice grid
    if !dice.is_empty() {
        let die_width = 8; // "[99]!→[9]✓ " = ~10 chars max
        let cols = (width / die_width).max(1);

        for chunk in dice.chunks(cols) {
            let line: Vec<String> = chunk.iter().map(format_die_entry).collect();
            out.push_str(&line.join("  "));
            out.push('\n');
        }
        out.push('\n');
    }

    // Summary
    if is_counted {
        // For counted results, show the actual count from the expression
        let total = dice.iter().filter(|d| is_dice_entry(d)).count();
        out.push_str(&format!(
            "{} {} (from {} dice)",
            "Count:".green().bold(),
            result.to_string().bold(),
            total
        ));
        out.push('\n');
    } else if let Some(expr_str) = compute_expression_display(dice) {
        out.push_str(&format!(
            "{} {} = {}",
            "Eval:".green().bold(),
            expr_str,
            result.to_string().bold()
        ));
        out.push('\n');
    } else {
        let kept: Vec<_> = dice.iter().filter(|d| d.kept && is_dice_entry(d)).collect();
        let dropped: Vec<_> = dice
            .iter()
            .filter(|d| !d.kept && is_dice_entry(d))
            .collect();

        if !kept.is_empty() {
            let vals: Vec<String> = kept.iter().map(|d| d.value.to_string()).collect();
            let sum: i32 = kept.iter().map(|d| d.value).sum();
            out.push_str(&format!(
                "{} {} = {}",
                "Kept:".green().bold(),
                vals.join(" + "),
                sum.to_string().bold()
            ));
            out.push('\n');
        }
        if !dropped.is_empty() {
            let vals: Vec<String> = dropped.iter().map(|d| d.value.to_string()).collect();
            let sum: i32 = dropped.iter().map(|d| d.value).sum();
            out.push_str(&format!(
                "{} {} = {}",
                "Drop:".red().bold(),
                vals.join(" + "),
                sum
            ));
            out.push('\n');
        }
    }

    out
}

fn format_die_entry(die: &DieEntry) -> String {
    // Separator: show operator with spaces
    if die.kind == Some(DieEntryKind::Separator) {
        let op = die.operator.as_deref().unwrap_or("");
        return format!(" {} ", op).cyan().to_string();
    }

    // GroupOpen/GroupClose: show parens
    if die.kind == Some(DieEntryKind::GroupOpen) {
        return "(".cyan().to_string();
    }
    if die.kind == Some(DieEntryKind::GroupClose) {
        return ")".cyan().to_string();
    }
    if die.kind == Some(DieEntryKind::PoolOpen) {
        return "{".cyan().to_string();
    }
    if die.kind == Some(DieEntryKind::PoolClose) {
        return "}".cyan().to_string();
    }
    if die.kind == Some(DieEntryKind::Pipe) {
        return " | ".cyan().to_string();
    }

    // Literal: show plain number without brackets or checkmark
    if die.kind == Some(DieEntryKind::Literal) {
        return format!("{}", die.value).cyan().to_string();
    }

    if let Some(ref chain) = die.chain {
        if chain.len() > 1 {
            // Format chain based on kind
            let separator = match &die.kind {
                Some(DieEntryKind::Explode) => "!→",
                Some(DieEntryKind::Compound) => "!!",
                Some(DieEntryKind::Reroll) | Some(DieEntryKind::RerollOnce) => "→",
                Some(DieEntryKind::MinCap) | Some(DieEntryKind::MaxCap) => "→",
                Some(DieEntryKind::Emphasis) => "→",
                _ => "→",
            };

            let chain_str: Vec<String> = chain.iter().map(|v| format!("[{}]", v)).collect();
            let chain_display = chain_str.join(separator);
            let status = if die.kept {
                "✓".green().to_string()
            } else {
                "✗".red().to_string()
            };
            format!("{}{}", chain_display, status)
        } else {
            // Single value chain (e.g., cap with same value)
            let value_str = format!("[{}]", die.value);
            if die.kept {
                format!("{}{}", value_str.green(), "✓".green())
            } else {
                format!("{}{}", value_str.red(), "✗".red())
            }
        }
    } else {
        // Simple die: [6]✓
        let value_str = format!("[{}]", die.value);
        if die.kept {
            format!("{}{}", value_str.green(), "✓".green())
        } else {
            format!("{}{}", value_str.red(), "✗".red())
        }
    }
}

fn terminal_width() -> usize {
    if let Some((Width(w), _)) = terminal_size() {
        w as usize
    } else {
        80
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_verbose_simple() {
        let dice = vec![
            DieEntry {
                value: 6,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 4,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 2,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
        ];
        let output = render_verbose("3d6", 12, &dice);
        assert!(output.contains("3d6 = 12"));
        assert!(output.contains("Kept:"));
    }

    #[test]
    fn test_render_verbose_keep_drop() {
        let dice = vec![
            DieEntry {
                value: 6,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 5,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 4,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 3,
                kept: false,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 2,
                kept: false,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 1,
                kept: false,
                chain: None,
                operator: None,
                kind: None,
            },
        ];
        let output = render_verbose("6d6k3", 15, &dice);
        assert!(output.contains("Kept:"));
        assert!(output.contains("Drop:"));
    }

    #[test]
    fn test_render_verbose_explode() {
        let dice = vec![DieEntry {
            value: 10,
            kept: true,
            chain: Some(vec![6, 4]),
            operator: None,
            kind: Some(DieEntryKind::Explode),
        }];
        let output = render_verbose("3d6!", 10, &dice);
        assert!(output.contains("6") && output.contains("4") && output.contains("→"));
        assert!(output.contains("Kept:"));
    }

    #[test]
    fn test_render_verbose_counted() {
        let dice = vec![
            DieEntry {
                value: 6,
                kept: true,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::Counted),
            },
            DieEntry {
                value: 5,
                kept: true,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::Counted),
            },
            DieEntry {
                value: 4,
                kept: false,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::Counted),
            },
        ];
        let output = render_verbose("3d6cs>=5", 2, &dice);
        assert!(output.contains("Count:"));
        assert!(output.contains("2"));
    }

    // ── Coverage Gap Tests ────────────────────────────────────────────

    #[test]
    fn test_render_verbose_literal() {
        let dice = vec![DieEntry {
            value: 42,
            kept: true,
            chain: None,
            operator: None,
            kind: Some(DieEntryKind::Literal),
        }];
        let output = render_verbose("42", 42, &dice);
        assert!(output.contains("42"));
        assert!(output.contains("Kept:"));
    }

    #[test]
    fn test_render_verbose_separator() {
        let dice = vec![
            DieEntry {
                value: 6,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 0,
                kept: false,
                chain: None,
                operator: Some("+".to_string()),
                kind: Some(DieEntryKind::Separator),
            },
            DieEntry {
                value: 4,
                kept: true,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::Literal),
            },
        ];
        let output = render_verbose("d6 + 4", 10, &dice);
        assert!(output.contains("+"));
        assert!(output.contains("10"));
    }

    #[test]
    fn test_render_verbose_reroll_chain() {
        let dice = vec![DieEntry {
            value: 4,
            kept: true,
            chain: Some(vec![1, 2, 4]),
            operator: None,
            kind: Some(DieEntryKind::Reroll),
        }];
        let output = render_verbose("d6 reroll on 1", 4, &dice);
        assert!(output.contains("→"));
        assert!(output.contains("Kept:"));
    }

    #[test]
    fn test_render_verbose_min_cap_chain() {
        let dice = vec![DieEntry {
            value: 3,
            kept: true,
            chain: Some(vec![1, 3]),
            operator: None,
            kind: Some(DieEntryKind::MinCap),
        }];
        let output = render_verbose("d6mi3", 3, &dice);
        assert!(output.contains("→"));
        assert!(output.contains("Kept:"));
    }

    #[test]
    fn test_render_verbose_grouped_binary_op() {
        // Simulate (2d6+1)*2 → GroupOpen [5] [1] + 1 GroupClose * 2
        let dice = vec![
            DieEntry {
                value: 0,
                kept: false,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::GroupOpen),
            },
            DieEntry {
                value: 5,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 1,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 0,
                kept: false,
                chain: None,
                operator: Some("+".to_string()),
                kind: Some(DieEntryKind::Separator),
            },
            DieEntry {
                value: 1,
                kept: true,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::Literal),
            },
            DieEntry {
                value: 0,
                kept: false,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::GroupClose),
            },
            DieEntry {
                value: 0,
                kept: false,
                chain: None,
                operator: Some("*".to_string()),
                kind: Some(DieEntryKind::Separator),
            },
            DieEntry {
                value: 2,
                kept: true,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::Literal),
            },
        ];
        let output = render_verbose("(2d6+1)*2", 14, &dice);
        assert!(output.contains("("));
        assert!(output.contains(")"));
        assert!(output.contains("Eval:"));
        assert!(output.contains("14"));
    }

    #[test]
    fn test_compute_expression_display_no_groups() {
        let dice = vec![
            DieEntry {
                value: 5,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 3,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 0,
                kept: false,
                chain: None,
                operator: Some("+".to_string()),
                kind: Some(DieEntryKind::Separator),
            },
            DieEntry {
                value: 4,
                kept: true,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::Literal),
            },
        ];
        assert!(compute_expression_display(&dice).is_none());
    }

    #[test]
    fn test_compute_expression_display_with_groups() {
        let dice = vec![
            DieEntry {
                value: 0,
                kept: false,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::GroupOpen),
            },
            DieEntry {
                value: 5,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 0,
                kept: false,
                chain: None,
                operator: Some("+".to_string()),
                kind: Some(DieEntryKind::Separator),
            },
            DieEntry {
                value: 1,
                kept: true,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::Literal),
            },
            DieEntry {
                value: 0,
                kept: false,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::GroupClose),
            },
            DieEntry {
                value: 0,
                kept: false,
                chain: None,
                operator: Some("*".to_string()),
                kind: Some(DieEntryKind::Separator),
            },
            DieEntry {
                value: 2,
                kept: true,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::Literal),
            },
        ];
        let result = compute_expression_display(&dice);
        assert_eq!(result, Some("(5 + 1) * 2".to_string()));
    }

    #[test]
    fn test_render_verbose_pool_grouped() {
        let dice = vec![
            DieEntry {
                value: 0,
                kept: false,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::PoolOpen),
            },
            DieEntry {
                value: 3,
                kept: false,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 6,
                kept: false,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 2,
                kept: false,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 6,
                kept: false,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 0,
                kept: false,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::Pipe),
            },
            DieEntry {
                value: 6,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 5,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 2,
                kept: true,
                chain: None,
                operator: None,
                kind: None,
            },
            DieEntry {
                value: 0,
                kept: false,
                chain: None,
                operator: None,
                kind: Some(DieEntryKind::PoolClose),
            },
        ];
        let output = render_verbose("{4d6, 3d8}kh", 13, &dice);
        assert!(output.contains("{"));
        assert!(output.contains("}"));
        assert!(output.contains("|"));
        assert!(output.contains("Kept:"));
    }
}
