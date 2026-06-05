/// Tool result trimming: reduce verbose output before Claude sees it.
/// Prevents context bloat from cargo build, large glob results, etc.
/// Max chars before triggering trim for bash output.
const BASH_TRIM_THRESHOLD: usize = 8_000;
/// Max chars to keep after trimming.
const BASH_TRIM_KEEP: usize = 4_000;

/// Max file paths before trimming glob results.
const GLOB_TRIM_THRESHOLD: usize = 100;
/// Max paths to keep after trimming.
const GLOB_TRIM_KEEP: usize = 30;

/// Check if bash tool response is verbose enough to trim.
/// Returns `Some(trimmed_output)` if trimming needed, None otherwise.
pub(crate) fn trim_bash_output(tool_response: &str) -> Option<String> {
    if tool_response.len() <= BASH_TRIM_THRESHOLD {
        return None;
    }

    let lines: Vec<&str> = tool_response.lines().collect();
    let total = lines.len();

    let head_count = 20.min(total);
    let remaining_budget = BASH_TRIM_KEEP.saturating_sub(head_count.saturating_mul(80));
    #[expect(
        clippy::integer_division,
        reason = "divisor 80 is literal non-zero constant"
    )]
    let tail_count = (remaining_budget / 80).min(total.saturating_sub(head_count));

    let head: Vec<&str> = lines
        .get(..head_count)
        .map_or_else(Vec::new, <[&str]>::to_vec);
    let tail: Vec<&str> = if tail_count > 0 {
        let start = total.saturating_sub(tail_count);
        lines.get(start..).map_or_else(Vec::new, <[&str]>::to_vec)
    } else {
        Vec::new()
    };

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "head_count and tail_count derived from lines.len() and provably bounded by total"
    )]
    let omitted = total.saturating_sub(head_count + tail_count);
    let mut result = head.join("\n");
    if omitted > 0 {
        use std::fmt::Write as _;
        writeln!(result, "\n... ({omitted} lines trimmed) ...\n").ok();
    }
    result.push_str(&tail.join("\n"));
    Some(result)
}

/// Check if glob results have too many file paths.
/// Returns Some(trimmed) if trimming needed, None otherwise.
pub(crate) fn trim_glob_output(tool_response: &str) -> Option<String> {
    use std::fmt::Write as _;
    let lines: Vec<&str> = tool_response.lines().collect();
    if lines.len() <= GLOB_TRIM_THRESHOLD {
        return None;
    }

    let total = lines.len();
    let kept: Vec<&str> = lines
        .get(..GLOB_TRIM_KEEP)
        .map_or_else(Vec::new, <[&str]>::to_vec);
    let omitted = total.saturating_sub(GLOB_TRIM_KEEP);

    let mut result = kept.join("\n");
    writeln!(result, "\n... and {omitted} more files (total: {total}). Narrow your glob pattern for focused results.").ok();
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_bash_not_trimmed() {
        assert!(trim_bash_output("hello world").is_none());
    }

    #[test]
    fn test_long_bash_trimmed() {
        let long_output = "warning: unused variable\n".repeat(500);
        let result = trim_bash_output(&long_output);
        assert!(result.is_some());
        assert!(result.as_ref().is_some_and(|r| r.contains("trimmed")));
    }

    #[test]
    fn test_short_glob_not_trimmed() {
        let lines = (0..50)
            .map(|i| format!("file_{i}.rs"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(trim_glob_output(&lines).is_none());
    }

    #[test]
    fn test_long_glob_trimmed() {
        let lines = (0..150)
            .map(|i| format!("src/file_{i}.rs"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = trim_glob_output(&lines);
        assert!(result.is_some());
        assert!(result.as_ref().is_some_and(|r| r.contains("more files")));
    }
}
