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
/// extension AND that resolves INSIDE the active governed workspace. Conservative
/// by construction: it scans whitespace/quote/redirect-delimited tokens for a
/// path that carries BOTH a source-tree segment AND a source extension, so a
/// benign `> /tmp/out.json` or `> Cargo.toml` does not trip it. A token needing
/// both signals keeps the deny scoped to the laundering case the gate exists to
/// stop.
///
/// JURISDICTION (the false-positive root cause this fixes): the deny exists ONLY
/// to stop a Bash write that LAUNDERS an Edit/Write past the pre-write gate — and
/// that gate's jurisdiction is the CURRENT workspace. An ABSOLUTE target path
/// that resolves OUTSIDE the workspace root (another project, `/tmp`, `/var`) is
/// not something the pre-write gate would ever mediate, so blocking it protects
/// nothing and only obstructs legitimate work. Such targets are passed through.
/// In-workspace relative paths (`crates/foo/src/lib.rs`, `src/main.rs`) and
/// in-workspace absolute paths still DENY. SOURCE:
/// github.com/anthropics/claude-code/issues/29709 (Bash launders Edit/Write);
/// path containment is the canonical guard (CWE-23 boundary, inverted).
pub(in crate::gates::pre_tool_bash) fn targets_tracked_source(cmd: &str) -> bool {
    // A `kavach`/`kavach db` invocation is an RPC, never a file write — a source
    // path inside its `--content "..."` argument is DATA, not a write target.
    // Exempt it so `kavach db write --content "...crates/x.rs..."` is not a false
    // positive. (The Python/sed/redirect bypasses this gate exists to catch are
    // not kavach calls, so detection of the real laundering case is unaffected.)
    if super::super::quote::is_kavach_cli(cmd) {
        return false;
    }
    let workspace_root = workspace_root();
    let lower = cmd.to_lowercase();
    tokenize_paths(&lower).any(|tok| {
        let in_source_tree = tok.contains("crates/")
            || tok.starts_with("src/")
            || tok.contains("/src/")
            || tok.contains("tests/");
        let has_source_ext = SOURCE_EXTS.iter().any(|ext| tok.ends_with(ext));
        in_source_tree
            && has_source_ext
            && in_workspace_jurisdiction(tok, workspace_root.as_deref())
    })
}

/// The active governed workspace root: the nearest ancestor of cwd holding a
/// `Cargo.toml`. `None` when cwd is not inside a Rust workspace (e.g. tests run
/// from a scratch dir) — then jurisdiction falls back to relative-only (below),
/// which preserves every in-tree true positive without a filesystem anchor.
fn workspace_root() -> Option<std::path::PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    cwd.ancestors()
        .find(|dir| dir.join("Cargo.toml").is_file())
        .map(std::path::Path::to_path_buf)
}

/// True when `tok` (a lowercased path token) is a write the pre-write gate would
/// govern — i.e. it lands INSIDE the workspace.
///
/// - A RELATIVE path (`crates/foo/src/lib.rs`, `src/main.rs`) resolves against
///   cwd, which the harness runs from the workspace, so it is in-jurisdiction.
/// - An ABSOLUTE path is in-jurisdiction ONLY if it is prefixed by the workspace
///   root. An absolute path elsewhere (another project, `/tmp`, `/var`) is OUT of
///   jurisdiction — the pre-write gate never mediates it, so the deny must not
///   fire. When the root is unknown, an absolute path is treated as external
///   (fail OPEN here: a false "external" only downgrades a deny to the benign
///   advisory; it never lets an in-repo launder through, since in-repo edits use
///   relative paths from the workspace cwd).
fn in_workspace_jurisdiction(tok: &str, workspace_root: Option<&std::path::Path>) -> bool {
    if !tok.starts_with('/') {
        return true; // relative → resolves under the workspace cwd
    }
    // Absolute path: in-jurisdiction only if prefixed by the workspace root.
    // `tok` is already lowercased by the caller, so compare lowercased. Unknown
    // root (None) ⇒ treat the absolute path as external (fail OPEN, see doc).
    workspace_root.is_some_and(|root| tok.starts_with(&root.to_string_lossy().to_lowercase()))
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
