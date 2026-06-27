//! Advisory: a legacy POSIX tool was used in Bash where a faster Rust toolbelt
//! equivalent exists. Advisory only (never a block) — the legacy tool may be the
//! only one present on a host. `grep`→`rg` is owned by `grep_guard`; this covers the
//! rest. Quote-aware via the parent's `strip_quoted_regions` so a tool name inside a
//! string is data, not a call. SOURCE: global CLAUDE.md §Toolbelt-is-law;
//! decision.engine.toolbelt-cli-advisory.
use crate::gates::pre_tool_bash::strip_quoted_regions;
/// (legacy command word, Rust replacement, one-line why). Matched as a command word
/// at a boundary so `find` in `findstr`/`oldfind` or an arg substring never fires.
const TOOLBELT: &[(&str, &str, &str)] = &[
    ("jq", "jaq", "drop-in jq syntax, faster startup"),
    ("sed", "sd", "literal find/replace, no regex foot-guns"),
    ("curl", "xh", "HTTPie-style, JSON by default"),
    ("find", "fd", "gitignore-aware, parallel"),
    ("du", "dust", "tree-sorted disk usage"),
    ("ps", "procs", "colored, human columns"),
    (
        "cat",
        "bat",
        "syntax highlight + line numbers (reads only — NEVER on .env)",
    ),
    ("diff", "difft", "structural (AST) diff"),
];
/// `Some(advisory)` when a legacy tool appears as a command word and its Rust
/// toolbelt replacement should be used. `None` otherwise.
pub(crate) fn check_toolbelt_cli(command: &str) -> Option<String> {
    let scrubbed = strip_quoted_regions(command);
    let hits: Vec<String> = TOOLBELT
        .iter()
        .filter(|(legacy, _, _)| is_command_word(&scrubbed, legacy))
        .map(|(legacy, rust, why)| format!("  `{legacy}` → `{rust}` ({why})"))
        .collect();
    if hits.is_empty() {
        return None;
    }
    Some(format!(
        "[ADVISORY:toolbelt] Prefer the Rust toolbelt for these — for JSON/file work in \
         a script ALWAYS use the Rust tool (it's the project default, §Toolbelt-is-law):\n{}",
        hits.join("\n")
    ))
}
/// True when `tool` appears as a command word: at the start of the command or right
/// after a `;`/`|`/`&`/`(` separator, and followed by whitespace/EOL — never as a
/// substring of another token (`oldfind`, `category`, `procstat`).
fn is_command_word(scrubbed: &str, tool: &str) -> bool {
    scrubbed
        .split(['|', ';', '&', '(', '\n'])
        .map(str::trim_start)
        .any(|seg| {
            seg.strip_prefix(tool)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
        })
}
#[cfg(test)]
#[path = "toolbelt_cli_test.rs"]
#[cfg(test)]
#[path = "toolbelt_cli_test.rs"]
mod tests;
