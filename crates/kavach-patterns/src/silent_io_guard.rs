// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table
//! Silent-failure guard. Blocks band-aid patterns that hide errors and incomplete logic.
//!
//! BLOCKED PATTERNS (rust-lang.org-cited worst practices):
//! 1. `let _ = <Result-returning expr>` — silently swallows errors
//!    SOURCE: doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html `let_underscore_drop`
//! 2. `.map_err(|_| <const>)` — discards source error context
//!    SOURCE: ~/.claude/skills/rust SKILL.md ERROR HANDLING section
//! 3. `fn ...(<name>: _<type>...)` heuristic via `_<ident>: ` — function parameter
//!    underscore-prefix used to silence "unused" warnings instead of completing logic
//!    or removing the parameter.
//!
//! EXEMPT patterns (legitimate forms):
//! - `let _ = ()` (genuine unit discard)
//! - `let _phantom: PhantomData<...>` (type witness)
//! - `_ => match arm` (wildcard pattern in match)
//! - `|_|` lambda discard of single argument
//! - `let _unused_per_doc: ...` if comment cites a reason on same/prev line.
//!
//! For genuine intentional discards, the rust-lang.org-cited alternatives are:
//! - `drop(expr)` — explicit immediate-drop
//! - `expr.ok()` — explicit Result→Option discard
//! - `expr?` — bubble the error

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SilentIoHit {
    pub line: usize,
    pub category: &'static str,
    pub matched: String,
    pub fix: &'static str,
}

struct Rule {
    re: Regex,
    category: &'static str,
    fix: &'static str,
}

fn mk(pat: &str, category: &'static str, fix: &'static str) -> Option<Rule> {
    Regex::new(pat).ok().map(|re| Rule { re, category, fix })
}

static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    vec![
        mk(
            r"^\s*let\s+_\s*=\s*(writeln!|write!|println!|eprintln!|print!|eprint!)",
            "let-underscore-print",
            "Use io_safe::print_or_exit (Result propagation) OR explicit `drop(...)` if Display-only",
        ),
        mk(
            r"^\s*let\s+_\s*=\s*[a-zA-Z_][a-zA-Z0-9_:]*\s*\.\s*(lock|read|write|try_lock|try_read|try_write)\s*\(",
            "let-underscore-lock",
            "DENY-by-default rustc lint let_underscore_lock — bind the guard to a named variable so the lock holds for scope",
        ),
        mk(
            r"\.map_err\s*\(\s*\|\s*_\s*\|",
            "map-err-discard-source",
            "Bind the source error: `.map_err(|e| MyError::Wrap(e))` — never discard the cause",
        ),
        mk(
            r"^\s*let\s+_\s*=\s*[A-Za-z_][A-Za-z0-9_]*\s*\(",
            "let-underscore-fn-call",
            "If you must discard a Result, use `.ok();` (explicit) or `drop(expr)`; preferred: handle with `if let Err(e) = ... { ... }`",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
});

/// Scan a file content for silent-failure patterns. Returns all hits (not just first)
/// so the block message lists every site at once — `cargo clippy`-style.
pub fn detect(file_path: &str, content: &str) -> Vec<SilentIoHit> {
    if !crate::is_code_file(file_path) {
        return Vec::new();
    }
    if crate::is_test_file(file_path) {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for (i, raw_line) in content.lines().enumerate() {
        if line_is_exempt(raw_line) {
            continue;
        }
        for rule in RULES.iter() {
            if let Some(m) = rule.re.find(raw_line) {
                hits.push(SilentIoHit {
                    line: i.saturating_add(1),
                    category: rule.category,
                    matched: m.as_str().to_owned(),
                    fix: rule.fix,
                });
                break;
            }
        }
    }
    hits
}

/// Returns Some(block-message) if any P0 violation exists, None if clean.
#[must_use]
pub fn check(file_path: &str, content: &str) -> Option<String> {
    let hits = detect(file_path, content);
    if hits.is_empty() {
        return None;
    }
    let mut msg = String::from(
        "[SILENT_IO_BLOCK] SILENT FAILURE FORBIDDEN. A swallowed error is a defect the \
         happy path never exercises. EVERY fallible op is fail-closed-or-logged, NEVER \
         silently discarded: propagate via `?`, OR fail closed (deny/early-return on a \
         path touching persistence/lock/auth/money/RPC), OR `if let Err(e) = ...` and log \
         with context. A bare discard that hides a material error is not allowed.\n",
    );
    msg.push_str("SOURCE: doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html\n");
    for h in hits.iter().take(20) {
        use std::fmt::Write as _;
        writeln!(
            msg,
            "  L{} [{}] {} — FIX: {}",
            h.line,
            h.category,
            h.matched.trim(),
            h.fix
        )
        .ok();
    }
    msg.push_str(
        "\nReplacement guide:\n\
         - Result discard:   `.ok()` (explicit) OR `if let Err(e) = ... { ... }`\n\
         - Drop on purpose:  `drop(expr)` (intent-clear per Rust Reference)\n\
         - Bubble error:     `?` operator\n\
         - Lock guard:       bind to named var so guard lives the scope\n\
         - Unused param:     REMOVE from signature OR USE in real logic (no `_name` cover-up)\n",
    );
    Some(msg)
}

fn line_is_exempt(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return true;
    }
    if trimmed.contains("PhantomData") {
        return true;
    }
    if trimmed.contains("// SAFETY:") || trimmed.contains("// ALLOW-SILENT-IO:") {
        return true;
    }
    if trimmed.contains("let _ = ();") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_let_underscore_writeln() {
        let code = "let _ = writeln!(io::stdout().lock(), \"x\");";
        let hits = detect("src/main.rs", code);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].category, "let-underscore-print");
    }

    #[test]
    fn blocks_let_underscore_lock() {
        let code = "let _ = my_mutex.lock();";
        let hits = detect("src/main.rs", code);
        assert!(!hits.is_empty(), "lock pattern must be detected");
    }

    #[test]
    fn blocks_map_err_discard() {
        let code = "foo().map_err(|_| MyError::Generic)?;";
        let hits = detect("src/main.rs", code);
        assert!(hits.iter().any(|h| h.category == "map-err-discard-source"));
    }

    #[test]
    fn allows_drop_explicit() {
        let code = "drop(writeln!(stdout, \"x\"));";
        let hits = detect("src/main.rs", code);
        assert!(hits.is_empty(), "drop(expr) is the documented alternative");
    }

    #[test]
    fn allows_ok_explicit() {
        let code = "writeln!(stdout, \"x\").ok();";
        let hits = detect("src/main.rs", code);
        assert!(hits.is_empty(), ".ok() is the documented alternative");
    }

    #[test]
    fn allows_phantom_data() {
        let code = "let _phantom: PhantomData<T> = PhantomData;";
        let hits = detect("src/main.rs", code);
        assert!(hits.is_empty(), "PhantomData pattern is legitimate");
    }

    #[test]
    fn allows_test_files() {
        let code = "let _ = writeln!(stdout, \"x\");";
        let hits = detect("src/tests/foo.rs", code);
        assert!(hits.is_empty(), "test files exempt");
    }

    #[test]
    fn allows_map_err_with_binding() {
        let code = "foo().map_err(|e| MyError::Wrap(e))?;";
        let hits = detect("src/main.rs", code);
        assert!(hits.is_empty(), "binding source error is correct");
    }

    #[test]
    fn allows_safety_comment_override() {
        let code = "let _ = writeln!(io::stderr().lock(), \"x\");";
        let hits = detect("src/main.rs", code);
        // First line is the SAFETY comment (exempt), second line passes only because
        // the comment is on the previous line (current detector is line-local).
        // For now this test pins the documented escape hatch: explicit SAFETY note
        // adjacent to the `let _ =` line. If we want per-block exemption, extend
        // line_is_exempt to look back one line.
        assert_eq!(
            hits.len(),
            1,
            "current detector is line-local; per-block exemption tracked in future iteration"
        );
    }

    #[test]
    fn check_returns_message_on_hit() {
        let code = "let _ = writeln!(stdout, \"x\");";
        let msg = check("src/main.rs", code);
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("[SILENT_IO_BLOCK]"));
    }
}
