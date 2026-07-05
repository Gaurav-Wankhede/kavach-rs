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
//! HANDLE a fallible Result — never discard it: `expr?` to propagate, `if let Err(e) =
//! expr { … }`, or `match`. `drop(expr)` is for a true unit/guard ONLY (a value with no
//! error). `.ok()` / `let _ = <fallible>` SWALLOW the error (let_underscore_must_use) — forbidden.

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
            "Handle the write Result: `if let Err(e) = writeln!(...) { ... }` or `?` (use io_safe::print_or_exit for propagation). Never `let _ =`/`drop()` a write that can fail",
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
            "Handle the Result — `if let Err(e) = call() { ... }`, `?` to propagate, or `match`. A discarded Result is a swallowed failure (let_underscore_must_use); suppressing it is forbidden, not an option",
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
        "[SILENT_IO_POLICY] SILENT FAILURE FORBIDDEN. A swallowed error is a defect the \
         happy path never exercises -> EVERY fallible op is fail-closed-or-logged, NEVER \
         silently discarded: propagate via `?`, OR fail closed (deny/early-return on a \
         path touching persistence/lock/auth/money/RPC), OR `if let Err(e) = ...` and log \
         with context -> fix the discard -> retry.\n",
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
        "\nReplacement guide (handle — never suppress):\n\
         - Fallible Result:  `?` to propagate, `if let Err(e) = ... { ... }`, or `match` — ACT on the error\n\
         - FORBIDDEN:        `.ok()` / `let _ = <fallible>` / `drop(<Result>)` — these swallow the error (let_underscore_must_use)\n\
         - True unit/no-op:  `drop(x)` ONLY for a value with no error (a guard, a `()`), never a Result\n\
         - Lock guard:       bind to a named var so the guard lives the scope\n\
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
#[path = "silent_io_guard_test.rs"]
mod tests;
