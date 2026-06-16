use colored::*;
use terminal_size::{terminal_size, Width};

/// Entry for a single die in verbose display
#[derive(Debug, Clone)]
pub struct DieEntry {
    pub value: i32,
    pub kept: bool,
    pub chain: Option<Vec<i32>>,
    pub operator: Option<String>, // e.g., "+4", "-1", "*2"
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

    // Dice grid
    if !dice.is_empty() {
        let die_width = 6; // "[99]✓ " = 6 chars max
        let cols = (width / die_width).max(1);

        for chunk in dice.chunks(cols) {
            let line: Vec<String> = chunk.iter().map(format_die_entry).collect();
            out.push_str(&line.join("  "));
            out.push('\n');
        }
        out.push('\n');
    }

    // Summary
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

    out
}

fn format_die_entry(die: &DieEntry) -> String {
    // If this entry has an operator (from arithmetic literal), show it specially
    if let Some(ref op) = die.operator {
        return format!("[{}{}]", op, die.value).cyan().to_string();
    }

    if let Some(ref chain) = die.chain {
        // Explode/reroll chain: [6]→[4]✓
        let chain_str: Vec<String> = chain.iter().map(|v| format!("[{}]", v)).collect();
        let chain_display = chain_str.join("→");
        let status = if die.kept {
            "✓".green().to_string()
        } else {
            "✗".red().to_string()
        };
        format!("{}{}", chain_display, status)
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
            },
            DieEntry {
                value: 4,
                kept: true,
                chain: None,
                operator: None,
            },
            DieEntry {
                value: 2,
                kept: true,
                chain: None,
                operator: None,
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
            },
            DieEntry {
                value: 5,
                kept: true,
                chain: None,
                operator: None,
            },
            DieEntry {
                value: 4,
                kept: true,
                chain: None,
                operator: None,
            },
            DieEntry {
                value: 3,
                kept: false,
                chain: None,
                operator: None,
            },
            DieEntry {
                value: 2,
                kept: false,
                chain: None,
                operator: None,
            },
            DieEntry {
                value: 1,
                kept: false,
                chain: None,
                operator: None,
            },
        ];
        let output = render_verbose("6d6k3", 15, &dice);
        assert!(output.contains("Kept:"));
        assert!(output.contains("Drop:"));
    }
}
