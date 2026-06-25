//! The self-audit matrix: four silent-failure / unsafe-mutation classes, scanned
//! line-by-line over a Rust source. Pure string heuristics (no AST) — kept
//! deliberately conservative to minimize false positives, with a `// doctor:ok`
//! line-suffix escape so a reviewed benign-by-design site is silenced explicitly
//! rather than the whole class being dropped.

/// The four audit classes. Each maps to a lived kavach incident class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Class {
    /// `let _ = <fallible>` — discards a Result/Option from an IO/DB call.
    SilentDiscard,
    /// `.ok()` swallow on a DB/IO call with no surrounding log.
    SwallowedResult,
    /// `DELETE`/`UPDATE` in a query string (destructive mutation to review).
    DestructiveQuery,
    /// `Err(_) =>` arm collapsing an error into a non-error value silently.
    SwallowedArm,
}

impl Class {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::SilentDiscard => "silent-discard",
            Self::SwallowedResult => "swallowed-result",
            Self::DestructiveQuery => "destructive-query",
            Self::SwallowedArm => "swallowed-arm",
        }
    }
}

/// One audit finding: class, file, 1-based line, and a short hint.
pub(super) struct Finding {
    pub(super) class: Class,
    pub(super) file: String,
    pub(super) line: usize,
    pub(super) hint: String,
}

/// Marker a reviewer adds to a line to silence a benign-by-design match.
const OK_MARKER: &str = "// doctor:ok";

/// IO/DB call fragments that make a swallowed Result genuinely risky (vs. a pure
/// in-memory parse, which is usually fine to ignore).
const IO_FRAGMENTS: [&str; 7] =
    ["output()", ".send(", ".query(", ".execute(", ".call(", ".persist(", "append_event"];

/// Scan one Rust source. `file` is the display path attached to each finding.
pub(super) fn scan_source(file: &str, src: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for (i, raw) in src.lines().enumerate() {
        let line_no = i.saturating_add(1);
        let line = strip_line_comment_for_code(raw);
        // An explicit reviewer escape silences any class on this line.
        if raw.contains(OK_MARKER) {
            continue;
        }
        if let Some(hint) = match_silent_discard(&line) {
            out.push(mk(Class::SilentDiscard, file, line_no, hint));
        }
        if let Some(hint) = match_swallowed_result(&line) {
            out.push(mk(Class::SwallowedResult, file, line_no, hint));
        }
        if let Some(hint) = match_destructive_query(file, &line) {
            out.push(mk(Class::DestructiveQuery, file, line_no, hint));
        }
        if let Some(hint) = match_swallowed_arm(&line) {
            out.push(mk(Class::SwallowedArm, file, line_no, hint));
        }
    }
    out
}

fn mk(class: Class, file: &str, line: usize, hint: &str) -> Finding {
    Finding { class, file: file.to_owned(), line, hint: hint.to_owned() }
}

/// Drop a trailing `//` line comment so comment prose can't trip a code matcher,
/// but keep string-literal content (queries live in strings). Conservative: only
/// strips when `//` is not inside an obvious `"` span on the same line.
fn strip_line_comment_for_code(line: &str) -> String {
    // `find("//")` yields a byte index at an ASCII boundary; `get(..idx)` is
    // panic-free (never splits a multi-byte char). Only strip when the `//` is
    // OUTSIDE a string literal (even number of quotes before it).
    line.find("//").map_or_else(
        || line.to_owned(),
        |idx| match line.get(..idx) {
            Some(head) if head.matches('"').count().is_multiple_of(2) => head.to_owned(),
            _ => line.to_owned(),
        },
    )
}

fn match_silent_discard(line: &str) -> Option<&'static str> {
    let t = line.trim_start();
    if t.starts_with("let _ =") && IO_FRAGMENTS.iter().any(|f| line.contains(f)) {
        return Some("`let _ =` discards a fallible IO/DB result — bind + handle or log");
    }
    None
}

fn match_swallowed_result(line: &str) -> Option<&'static str> {
    // `<io>().ok()` with no `?`/`warn`/`expect` on the line: a swallow.
    let has_io = IO_FRAGMENTS.iter().any(|f| line.contains(f));
    if has_io && line.contains(").ok()") && !line.contains("warn") {
        return Some("`.ok()` swallows a DB/IO error — log or propagate the Err");
    }
    None
}

fn match_destructive_query(file: &str, line: &str) -> Option<&'static str> {
    // Precision (kavach-doctor-FP-tighten): the read-back-assertion class targets
    // unbounded row DELETION only. Exclude the false-positive families that
    // drowned the real signal:
    //  - UPDATE: mutates fields, not the unbounded-delete class — not flagged.
    //  - test files: a fixture's DELETE is not a production mutation.
    //  - the SQL-guard's OWN banned-pattern literals (it stores "DELETE " as data
    //    to detect): a self-match, not a destructive op.
    //  - bounded deletes keyed by an id/key/pid WHERE clause: already targeted.
    if is_test_file(file) || is_sql_detector_source(file) {
        return None;
    }
    // Must be a real DELETE *statement*: `"DELETE <table>"` or `"DELETE $rec"`.
    // This excludes error-message / data strings that merely contain the word
    // "DELETE" (e.g. `"… after DELETE (partial)"`, `"serves; DELETE roadmap"`).
    if !is_delete_statement(line) {
        return None;
    }
    // Bounded (dynamic, not a frozen name-list): a WHERE that binds any `$param` is scoped by it (`=`/`CONTAINS`/edge).
    let keyed_where = line.contains("WHERE") && line.contains('$');
    // A record-id delete (`DELETE $pid`/`DELETE $ids`) targets a row by id.
    let record_id_delete = line.contains("DELETE $");
    // `RETURN BEFORE` returns the deleted rows — the count→delete→verify read-back is present, removal is verified.
    let verified_readback = line.contains("RETURN BEFORE");
    if keyed_where || record_id_delete || verified_readback {
        return None;
    }
    Some("unbounded DELETE — add a read-back assertion (count→delete→re-count) or a key predicate")
}

/// True iff the line opens a `DELETE` SQL statement inside a string literal —
/// `"DELETE <ident>` or `"DELETE $rec`, NOT the word DELETE buried in prose.
fn is_delete_statement(line: &str) -> bool {
    line.contains("\"DELETE ") || line.contains("\" DELETE ") || line.contains("query(\"DELETE")
}

/// A test source: findings here are fixtures, not production mutations. Covers
/// both `foo_test.rs`/`foo_tests.rs` and inline `tests.rs`/`test.rs` module files
/// and any path under a `tests/`/`test/` dir.
fn is_test_file(file: &str) -> bool {
    file.ends_with("_test.rs")
        || file.ends_with("_tests.rs")
        || file.ends_with("/tests.rs")
        || file.ends_with("/test.rs")
        || file.contains("/tests/")
        || file.contains("/test/")
}

/// kavach's OWN SQL-destructive guards store `"DELETE "` / `"UPDATE "` as
/// detection DATA — scanning them for those tokens is a self-match.
fn is_sql_detector_source(file: &str) -> bool {
    file.ends_with("sql_destructive.rs")
        || file.contains("write_bypass/")
        || file.ends_with("destructive_cli_guard.rs")
}

fn match_swallowed_arm(line: &str) -> Option<&'static str> {
    let t = line.replace(' ', "");
    let collapses = t.contains("Err(_)=>None")
        || t.contains("Err(_)=>{}")
        || t.contains("Err(_)=>()")
        || t.contains("Err(_)=>returnNone");
    if collapses && !line.contains("warn") {
        return Some("`Err(_) =>` collapses an error silently — match Err(e) and log it");
    }
    None
}
