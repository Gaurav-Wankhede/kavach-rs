//! Line-regex rule table for the design-patterns guard. Split out of
//! `design_patterns_guard.rs` to keep each module under the 100-LOC bar.
//!
//! SOURCES (verified 2026-06):
//! - <https://rust-unofficial.github.io/patterns/>
//! - <https://refactoring.guru/design-patterns/rust>

use crate::design_patterns_guard::PatternSeverity;
use regex::Regex;
use std::sync::LazyLock;

pub(crate) struct Rule {
    // `Option` so the `LazyLock` initializer never needs unwrap/expect (both
    // `forbid` at the workspace level). `None` is unreachable for these const
    // patterns; a `None` rule is simply skipped at match time.
    pub(crate) re: &'static Option<Regex>,
    pub(crate) sev: PatternSeverity,
    pub(crate) pattern: &'static str,
    pub(crate) fix: &'static str,
}

static RE_DENY_WARNINGS: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"#!?\[deny\(\s*warnings\s*\)\]").ok());

static RE_STRING_PARAM: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\bfn\s+\w+\s*\([^)]*&\s*String\b").ok());

static RE_VEC_PARAM: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\bfn\s+\w+\s*\([^)]*&\s*Vec\s*<").ok());

static RE_BOX_PARAM: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\bfn\s+\w+\s*\([^)]*&\s*Box\s*<").ok());

static RE_DEREF_IMPL: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"impl\s+(?:std::ops::)?Deref\s+for\s+\w+\s*\{").ok());

// GoF Singleton — `static mut` is unsound (data race, UB); idiomatic global is
// OnceLock/LazyLock. <https://refactoring.guru/design-patterns/singleton/rust/example>
static RE_STATIC_MUT: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\bstatic\s+mut\s+\w+").ok());

// GoF State/Strategy — `match self.<field>` is the behaviour-by-field smell the
// State/Strategy patterns remove. <https://refactoring.guru/design-patterns/state/rust/example>
static RE_STATE_MATCH: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\bmatch\s+self\.(?:state|mode|status|phase)\b").ok());

// GoF Iterator — a manual `i += 1` index walk reimplements Iterator and is an
// off-by-one magnet. <https://refactoring.guru/design-patterns/iterator/rust/example>
static RE_MANUAL_INDEX: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\b\w+\s*\+=\s*1\s*;").ok());

// GoF Observer — a hand-rolled callback list. Observer (fn-ptr/channel registry)
// makes subscribe/notify explicit. <https://refactoring.guru/design-patterns/observer/rust/example>
static RE_OBSERVER_LIST: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"Vec\s*<\s*Box\s*<\s*dyn\s+Fn").ok());

pub(crate) static RULES: LazyLock<Vec<Rule>> = LazyLock::new(build_rules);

fn build_rules() -> Vec<Rule> {
    use PatternSeverity::{P1Advisory, P2Warning};
    vec![
        Rule {
            re: &RE_DENY_WARNINGS,
            sev: P1Advisory,
            pattern: "deny(warnings) anti-pattern",
            fix: "Use RUSTFLAGS=\"-D warnings\" in CI or deny specific lints.",
        },
        Rule {
            re: &RE_STRING_PARAM,
            sev: P1Advisory,
            pattern: "borrowed-owned param: &String",
            fix: "Use &str — accepts both String and string literals via deref coercion.",
        },
        Rule {
            re: &RE_VEC_PARAM,
            sev: P1Advisory,
            pattern: "borrowed-owned param: &Vec<T>",
            fix: "Use &[T] — accepts Vec, arrays, and slices via deref coercion.",
        },
        Rule {
            re: &RE_BOX_PARAM,
            sev: P1Advisory,
            pattern: "borrowed-owned param: &Box<T>",
            fix: "Use &T — Box already provides indirection, double-borrow is wasteful.",
        },
        Rule {
            re: &RE_DEREF_IMPL,
            sev: P2Warning,
            pattern: "impl Deref — verify pointer-like semantics",
            fix: "Deref is for smart pointers. If faking inheritance, use traits + composition.",
        },
        Rule {
            re: &RE_STATIC_MUT,
            sev: P1Advisory,
            pattern: "Singleton via static mut — unsound",
            fix: "Use OnceLock/LazyLock for a panic-free, thread-safe Singleton (no static mut, no unsafe).",
        },
        Rule {
            re: &RE_STATE_MATCH,
            sev: P2Warning,
            pattern: "State/Strategy: behaviour switched on self.<field>",
            fix: "match on a status field grows with each case. Encode states as a State enum or Strategy trait.",
        },
        Rule {
            re: &RE_MANUAL_INDEX,
            sev: P2Warning,
            pattern: "Iterator: manual index walk",
            fix: "Hand-rolled `i += 1` over a slice is an off-by-one magnet. Use Iterator + adapters (.iter()/.enumerate()/.zip()).",
        },
        Rule {
            re: &RE_OBSERVER_LIST,
            sev: P2Warning,
            pattern: "Observer: hand-rolled callback list",
            fix: "A bare Vec<Box<dyn Fn>> can't unsubscribe by identity. Use the Observer pattern: keyed registry or channel fan-out.",
        },
    ]
}
