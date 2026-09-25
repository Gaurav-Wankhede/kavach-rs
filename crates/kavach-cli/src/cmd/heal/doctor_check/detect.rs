//! The self-audit matrix: four silent-failure / unsafe-mutation classes, scanned
//! line-by-line over a Rust source. Trivia-aware matching via segment views
//! (code vs string-literal innards vs comments): code matchers never see prose
//! or string text, and query matchers see only string contents — with a
//! `// doctor:ok` line-suffix escape so a reviewed benign-by-design site is
//! silenced explicitly rather than the whole class being dropped.

/// The four audit classes. Each maps to a lived kavach incident class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Class {
    /// `let _ = <fallible>` — discards a Result/Option from an IO/DB call.
    SilentDiscard,
    /// `.ok()` swallow on a DB/IO call with no surrounding log.
    SwallowedResult,
    /// `DELETE`/`UPDATE` in a query string (destructive mutation to review).
    DestructiveQuery,
    /// `Err(_) =>` arm collapsing an error into a non-error value silently.
    SwallowedArm,
    /// Database integrity check error (PRAGMA integrity_check).
    DbHealth,
    /// Database WAL size exceeded threshold (>64MB).
    WalWarning,
}

impl Class {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::SilentDiscard => "silent-discard",
            Self::SwallowedResult => "swallowed-result",
            Self::DestructiveQuery => "destructive-query",
            Self::SwallowedArm => "swallowed-arm",
            Self::DbHealth => "db-health",
            Self::WalWarning => "wal-warning",
        }
    }
}

/// One audit finding: class, file, 1-based line, and a short hint.
pub(crate) struct Finding {
    pub class: Class,
    pub file: String,
    pub line: usize,
    pub hint: String,
}

/// Marker a reviewer adds to a line to silence a benign-by-design match.
const OK_MARKER: &str = "// doctor:ok";

/// IO/DB call fragments that make a swallowed Result genuinely risky (vs. a pure
/// in-memory parse, which is usually fine to ignore).
const IO_FRAGMENTS: [&str; 7] = [
    "output()",
    ".send(",
    ".query(",
    ".execute(",
    ".call(",
    ".persist(",
    "append_event",
];

/// Scan one Rust source. `file` is the display path attached to each finding.
pub(crate) fn scan_source(file: &str, src: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for (i, raw) in joined_logical_lines(src).into_iter().enumerate() {
        let line_no = i.saturating_add(1);
        // Segment views: code matchers see executable code only (comments and
        // string text are opaque), query matchers see string innards only.
        let code = code_view(&raw);
        let strs = str_view(&raw);
        // An explicit reviewer escape silences any class on this line.
        if raw.contains(OK_MARKER) {
            continue;
        }
        if let Some(hint) = match_silent_discard(&code) {
            out.push(mk(Class::SilentDiscard, file, line_no, hint));
        }
        if let Some(hint) = match_swallowed_result(&code) {
            out.push(mk(Class::SwallowedResult, file, line_no, hint));
        }
        if let Some(hint) = match_destructive_query(file, &strs) {
            out.push(mk(Class::DestructiveQuery, file, line_no, hint));
        }
        if let Some(hint) = match_swallowed_arm(&code) {
            out.push(mk(Class::SwallowedArm, file, line_no, hint));
        }
    }
    out
}

/// Fold `\`-continued string-literal lines into one logical line so a multi-line
/// query's WHERE `$param` / `RETURN BEFORE` isn't split across physical lines.
fn joined_logical_lines(src: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut pending: Option<String> = None;
    for raw in src.lines() {
        let mut cur = pending.take().unwrap_or_default();
        cur.push_str(raw);
        if raw.trim_end().ends_with('\\') {
            pending = Some(cur);
        } else {
            out.push(cur);
        }
    }
    if let Some(rest) = pending {
        out.push(rest);
    }
    out
}

fn mk(class: Class, file: &str, line: usize, hint: &str) -> Finding {
    Finding {
        class,
        file: file.to_owned(),
        line,
        hint: hint.to_owned(),
    }
}

/// Code-only view of a logical line: comments and string literals dropped, so
/// comment prose and string text can never trip a code matcher. Replaces the
/// old quote-counting `//` stripper (which kept block comments and mishandled
/// `//` inside strings with an odd quote count).
fn code_view(line: &str) -> String {
    use kavach_patterns::lex::{RsKind, segments, text};
    segments(line)
        .iter()
        .filter(|s| s.kind == RsKind::Code)
        .map(|s| text(line, s))
        .collect()
}

/// String-literal innards of a logical line, joined: what query matchers scan.
/// Delimiters (`"`, `r#"…"#`, `b"…"`, char quotes) are stripped; everything
/// else — including comment-looking text inside strings — is preserved.
fn str_view(line: &str) -> String {
    use kavach_patterns::lex::{RsKind, segments, text};
    segments(line)
        .iter()
        .filter(|s| s.kind == RsKind::Str)
        .map(|s| str_inner(text(line, s)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Strip literal delimiters, returning the content slice (borrowed, no alloc).
fn str_inner(lit: &str) -> &str {
    let mut s = lit;
    if let Some(rest) = s.strip_prefix('b') {
        s = rest;
    }
    if let Some(rest) = s.strip_prefix('r') {
        s = rest;
    }
    while let Some(rest) = s.strip_prefix('#') {
        s = rest;
    }
    if let Some(rest) = s.strip_prefix('"').or_else(|| s.strip_prefix('\'')) {
        s = rest;
    }
    while let Some(rest) = s.strip_suffix('#') {
        s = rest;
    }
    if let Some(rest) = s.strip_suffix('"').or_else(|| s.strip_suffix('\'')) {
        s = rest;
    }
    s
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

fn match_destructive_query(file: &str, strs: &str) -> Option<&'static str> {
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
    // Must be a real DELETE *statement* at the start of string content:
    // `DELETE <table>` or `DELETE $rec`. Error-message / data strings that
    // merely contain the word ("… after DELETE (partial)") never start with it.
    if !is_delete_statement(strs) {
        return None;
    }
    // Bounded if the WHERE binds any `$param` (dynamic, not a frozen name-list).
    let keyed_where = strs.contains("WHERE") && strs.contains('$');
    // A record-id delete (`DELETE $pid`) targets a row by id.
    let record_id_delete = strs.contains("DELETE $");
    // `RETURN BEFORE` returns deleted rows — the read-back is present, not silent.
    let verified_readback = strs.contains("RETURN BEFORE");
    if keyed_where || record_id_delete || verified_readback {
        return None;
    }
    Some("unbounded DELETE — add a read-back assertion (count→delete→re-count) or a key predicate")
}

/// True iff string-literal content opens a `DELETE` SQL statement —
/// `DELETE <ident>` or `DELETE $rec`, NOT the word DELETE buried in prose.
fn is_delete_statement(strs: &str) -> bool {
    let head = strs.trim_start();
    head.starts_with("DELETE ") || head.starts_with("DELETE $")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_finds_silent_discard() {
        let src = r#"
fn foo() {
    let _ = db.query("SELECT 1"); // silent failure - result not checked
}
"#;
        let findings = scan_source("test.rs", src);
        assert!(!findings.is_empty());
    }

    #[test]
    fn doctor_ignores_doctor_ok() {
        let src = r#"
fn foo() {
    let _ = db.query("SELECT 1"); // doctor:ok
}
"#;
        let findings = scan_source("test.rs", src);
        assert!(findings.is_empty());
    }

    #[test]
    fn io_fragment_inside_a_string_is_not_a_swallow() {
        // Old code matched `.query(` inside the string literal: `log` returns
        // () here, so there is no swallowed IO Result.
        let src = "fn foo() {\n    let _ = log(\"query() failed\");\n}\n";
        let findings = scan_source("src/a.rs", src);
        assert!(
            findings.is_empty(),
            "got: {:?}",
            findings.iter().map(|f| f.class.label()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn warn_inside_a_string_does_not_silence_a_real_swallow() {
        // Old code saw "warn" inside the string and exempted the line (false
        // negative); the code view has no warn, so this must still hit.
        let src = "fn foo() {\n    db.query(\"warn: slow\").ok();\n}\n";
        let findings = scan_source("src/a.rs", src);
        assert!(
            findings.iter().any(|f| f.class == Class::SwallowedResult),
            "got: {:?}",
            findings.iter().map(|f| f.class.label()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn delete_in_comment_prose_is_not_a_query() {
        let src = "fn foo() {\n    // DELETE everything manually before deploy\n}\n";
        let findings = scan_source("src/a.rs", src);
        assert!(
            findings.iter().all(|f| f.class != Class::DestructiveQuery),
            "got: {:?}",
            findings.iter().map(|f| f.class.label()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn delete_mid_string_is_not_a_statement() {
        // Error/data prose mentioning DELETE must not flag — only content
        // OPENING with DELETE is a statement.
        let src = "fn foo() {\n    let e = \"failed after DELETE (partial)\";\n}\n";
        let findings = scan_source("src/a.rs", src);
        assert!(
            findings.iter().all(|f| f.class != Class::DestructiveQuery),
            "got: {:?}",
            findings.iter().map(|f| f.class.label()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn real_delete_statement_still_flags() {
        let src = "fn foo() {\n    db.exec(\"DELETE FROM events\");\n}\n";
        let findings = scan_source("src/a.rs", src);
        assert!(
            findings.iter().any(|f| f.class == Class::DestructiveQuery),
            "statement must flag: {:?}",
            findings.iter().map(|f| f.class.label()).collect::<Vec<_>>()
        );
    }
}
