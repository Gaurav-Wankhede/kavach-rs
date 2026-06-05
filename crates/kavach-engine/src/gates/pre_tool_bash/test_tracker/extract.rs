//! Command-position detection of long-running cargo jobs (`test`/`nextest`/
//! `build`/`check`) that share the `target/` artifact lock.
//!
//! FIX [CWE-184 over-broad-trigger]: the phrase inside another tool's quoted
//! arg is data, not a cargo invocation. Detection parses shell tokens in
//! command position (quote-aware) instead of a raw substring `.contains`.
//! RESEARCH: <https://cwe.mitre.org/data/definitions/184.html> ; POSIX XCU §2 —
//! quoted text is literal, never a token boundary. The `words` tokenizer + a
//! quote-stripped split keep a literal `|` inside a quoted arg from shattering
//! the command into a fake `cargo` segment.
mod words;

use words::segment_words;

/// Long-running cargo subcommands that share ONE `target/` artifact lock, so a
/// duplicate concurrent invocation on the same `-p` crate just blocks on the lock
/// ("Blocking waiting for file lock") and wastes a shell. `test`/`nextest` (the
/// 10-20 min workspace risk) AND `build`/`check` (slow release/all-targets
/// compiles) all qualify — the hazard is subcommand-agnostic.
/// SOURCE: <https://doc.rust-lang.org/cargo/guide/build-cache.html> (shared lock).
const TRACKED_SUBCOMMANDS: &[&str] = &["test", "nextest", "build", "check"];

/// True when `segment`'s command word is `cargo` and its first non-flag
/// subcommand is one of [`TRACKED_SUBCOMMANDS`]. Skips `!`, `time`, `VAR=val`.
fn segment_is_tracked_cargo_job(segment: &str) -> bool {
    let words = segment_words(segment);
    let mut it = words.iter().skip_while(|w| {
        w.as_str() == "!"
            || w.as_str() == "time"
            || w.find('=').is_some_and(|eq| {
                let lhs = w.get(..eq).unwrap_or("");
                !lhs.is_empty()
                    && !lhs.contains('/')
                    && lhs.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
    });
    let Some(cmd_word) = it.next() else {
        return false;
    };
    if cmd_word.rsplit('/').next().unwrap_or(cmd_word) != "cargo" {
        return false;
    }
    it.find(|w| !w.starts_with('-'))
        .is_some_and(|sub| TRACKED_SUBCOMMANDS.contains(&sub.as_str()))
}

/// Resolve the `-p <crate>` / `--package=<crate>` target, or `__workspace__`.
fn resolve_crate_key(words: &[String]) -> String {
    if words.iter().any(|w| w == "--workspace") {
        return "__workspace__".into();
    }
    let mut it = words.iter();
    while let Some(w) = it.next() {
        if (w == "-p" || w == "--package")
            && let Some(name) = it.next()
        {
            return name.clone();
        } else if let Some(rest) = w.strip_prefix("--package=") {
            return rest.to_owned();
        }
    }
    "__workspace__".into()
}

/// Crate/package name from a long-running cargo job (`test`/`nextest`/`build`/
/// `check`), or `None` if the command is not such an invocation in command
/// position. `__workspace__` when no `-p`/`--package` scopes it.
pub(super) fn extract_cargo_job_key(cmd: &str) -> Option<String> {
    let cmd = cmd.trim();
    // Command substitution can move a quoted arg into command position —
    // fail-closed-as-no-block (a missed test scope is recoverable; a false
    // P0 with no escape is worse). Mirrors legacy_tool_guard.
    if cmd.contains("$(") || cmd.contains('`') || cmd.contains("<(") {
        return None;
    }
    // Split on a quote-stripped view so a literal `|` inside a quoted arg
    // does not shatter the command into a fake `cargo nextest` segment.
    let stripped = super::super::strip_quoted_regions(cmd);
    if !stripped
        .split(['|', ';', '&'])
        .any(segment_is_tracked_cargo_job)
    {
        return None;
    }
    Some(resolve_crate_key(&segment_words(cmd)))
}
