//! Shared quote-aware primitive + the kavach-CLI fast-path predicate.

/// Replace each single/double-quoted span with ONE inert placeholder token.
///
/// Shared quote-aware primitive for the Bash-guard submodules: a trigger token
/// (`-p`, `cargo test`, `git commit`, `psql`, …) that lives *inside* a quoted
/// argument is data, not a command-position invocation. Collapsing each quoted
/// span to a single sentinel kills that CWE-184 false-positive class at the root.
///
/// The span becomes one `_` token, NOT blanks: a quoted value must still occupy
/// its argument *slot* so a preceding value-flag (`-E '…'`) consumes the
/// placeholder rather than the next real `-p`. Backslash-escaped quotes are
/// ordinary chars so they don't desync the quote state.
// §RADIUS-INTEGRITY: pub(crate) so sibling gate modules (prod_guard, future
// callers) route detection through the same quote-aware primitive.
pub(crate) fn strip_quoted_regions(cmd: &str) -> String {
    let mut out = String::with_capacity(cmd.len());
    let mut quote: Option<char> = None;
    let mut prev_backslash = false;
    for ch in cmd.chars() {
        match quote {
            Some(q) => {
                if ch == q && !prev_backslash {
                    quote = None;
                } else if ch.is_whitespace() {
                    // keep intra-span whitespace OUT so the span stays one token.
                }
            }
            None => {
                if (ch == '"' || ch == '\'') && !prev_backslash {
                    quote = Some(ch);
                    out.push('_');
                } else {
                    out.push(ch);
                }
            }
        }
        prev_backslash = ch == '\\' && !prev_backslash;
    }
    out
}

/// Return true for kavach CLI invocations — internal bookkeeping, no enforcement.
/// Matches: `kavach <subcommand>`, `~/.local/bin/kavach <subcommand>`.
pub(super) fn is_kavach_cli(cmd: &str) -> bool {
    let t = cmd.trim();
    t.starts_with("kavach ")
        || t.starts_with("~/.local/bin/kavach ")
        || t.contains("/kavach db ")
        || t.contains("/kavach status")
        || t.contains("/kavach rag ")
        || t.contains("/kavach rules ")
        || t.contains("/kavach session ")
        || t.contains("/kavach gates ")
}
