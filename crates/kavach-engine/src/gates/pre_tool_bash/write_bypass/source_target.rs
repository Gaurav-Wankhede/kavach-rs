//! Decide whether a write-bypass command targets a TRACKED SOURCE file — the
//! one case the bypass must be DENIED, not merely flagged.
//!
//! A Bash file-write (`python3 - <<EOF`, `> file`, `sed -i`, `tee`) sidesteps
//! the `pre-write` research / anti-pattern gate that fires only on Write/Edit.
//! For generated artifacts (configs, `loop.yaml`, `nextest.toml`) that dodge is
//! benign and stays advisory. For a hand-edited Rust SOURCE file it is
//! capability-laundering: the change must go through Write/Edit so the gate can
//! mediate. This module isolates the narrow "is the target a source file?"
//! predicate so the deny stays scoped and its false-positive surface auditable.
//! SOURCE: github.com/liberzon/claude-hooks (decompose; match each token)

#[cfg(test)]
#[path = "source_target_test.rs"]
mod tests;

/// File extensions that are hand-authored source the `pre-write` gate guards.
/// Generated/data formats (`.json .yaml .toml .lock .md`) are intentionally
/// EXCLUDED — writing those via Bash is the benign advisory case.
const SOURCE_EXTS: &[&str] = &[".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".sql"];

/// True when the command appears to write a tracked SOURCE file — a path under
/// a `crates/`, `src/`, or `tests/` directory whose name ends in a source
/// extension. Conservative by construction: it scans whitespace/quote/redirect-
/// delimited tokens for a path that carries BOTH a source-tree segment AND a
/// source extension, so a benign `> /tmp/out.json` or `> Cargo.toml` does not
/// trip it. A token needing both signals keeps the deny scoped to the
/// laundering case the gate exists to stop.
pub(in crate::gates::pre_tool_bash) fn targets_tracked_source(cmd: &str) -> bool {
    // A `kavach`/`kavach db` invocation is an RPC, never a file write — a source
    // path inside its `--content "..."` argument is DATA, not a write target.
    // Exempt it so `kavach db write --content "...crates/x.rs..."` is not a false
    // positive. (The Python/sed/redirect bypasses this gate exists to catch are
    // not kavach calls, so detection of the real laundering case is unaffected.)
    if super::super::quote::is_kavach_cli(cmd) {
        return false;
    }
    let lower = cmd.to_lowercase();
    tokenize_paths(&lower).any(|tok| {
        let in_source_tree = tok.contains("crates/")
            || tok.starts_with("src/")
            || tok.contains("/src/")
            || tok.contains("tests/");
        let has_source_ext = SOURCE_EXTS.iter().any(|ext| tok.ends_with(ext));
        in_source_tree && has_source_ext
    })
}

/// Split a command into candidate path tokens. Paths can be wrapped in quotes,
/// abut a redirect (`>file`), or sit among `open('crates/...','w')` arguments —
/// so we break on shell + Python punctuation, NOT just whitespace, and keep
/// `/`, `.`, `_`, `-` which a path needs.
fn tokenize_paths(lower: &str) -> impl Iterator<Item = &str> {
    const DELIMS: &[char] = &[
        ' ', '\t', '\n', '\'', '"', '(', ')', ',', '>', '<', '|', ';', '&', '=', '`',
    ];
    lower
        .split(|c: char| DELIMS.contains(&c))
        .map(str::trim)
        .filter(|t| !t.is_empty() && t.contains('/'))
}
