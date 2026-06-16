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
        // For counted results, show count instead of sum
        let count = dice.iter().filter(|d| d.kept).count();
        let total = dice.len();
        out.push_str(&format!(
            "{} {} (from {} dice)",
            "Count:".green().bold(),
            count.to_string().bold(),
            total
        ));
        out.push('\n');
    } else {
        let kept: Vec<_> = dice.iter().filter(|d| d.kept).collect();
        let dropped: Vec<_> = dice.iter().filter(|d| !d.kept).collect();

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
    // If this entry has an operator (from arithmetic literal), show it specially
    if let Some(ref op) = die.operator {
        return format!("[{}{}]", op, die.value).cyan().to_string();
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
}
