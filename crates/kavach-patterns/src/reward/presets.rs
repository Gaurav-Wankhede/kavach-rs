//! Built-in per-stack reward rubrics.
//!
//! A project names one via its `gate.reward_rubric` row, or relies on the Rust
//! default. Each preset encodes the verify commands + vacuous-test shape that
//! stack actually emits, so the RLAIF scores a `bun test` / `pytest` / `dx
//! bundle` the same way it scores a `cargo nextest` — no stack is silently zero.

use super::rubric::{EventClass, RewardRubric, SignalRule};
use regex::Regex;

// Universal debits — present in EVERY rubric regardless of stack. Gate-block and
// deferral-handoff are stack-independent failure classes (RCA
// mistake.false-fence-handoff-2026-06-17). Weights kept identical to the original
// scalar scorer so existing behavior is preserved.
pub(super) const GATE_BLOCK_WEIGHT: i64 = -6;
pub(super) const DEFERRAL_WEIGHT: i64 = -12;

// Phase-4 enriched UNIVERSAL signals (stack-independent quality, operator directive
// 2026-06-17 "consider more parameters"). Detectable from event text in any
// language, so they live in `with_universal` and apply to every rubric.
/// A shipped stub / placeholder (`todo!`, `unimplemented!`, `TODO:`, `FIXME`,
/// `pass  # stub`, `throw new Error("not implemented")`) — incomplete work.
const STUB_SHIPPED_WEIGHT: i64 = -5;
/// A swallowed error (`unwrap_or_default`, `let _ =`-discarded Result, empty
/// `catch {}` / `except: pass`) — a silent-failure quality debit.
const SILENT_FAILURE_WEIGHT: i64 = -3;
/// A documented root-cause block (`[RCA]`, `ROOT CAUSE`, `5-why`) — a quality
/// credit: the turn traced a symptom to its origin rather than surface-patching.
const RCA_PRESENT_WEIGHT: i64 = 2;

fn rule(id: &'static str, class: EventClass, pat: &str, weight: i64) -> Option<SignalRule> {
    Regex::new(pat).ok().map(|pattern| SignalRule {
        id,
        applies_to: class,
        pattern,
        weight,
    })
}

/// The deferral-handoff regex — shared by every stack (universal debit).
pub(super) const fn deferral_pattern() -> &'static str {
    r"(?i)the next (?:step|move) is yours|start a new session|in (?:a|another) (?:new )?session|you should run|over to you|let me know if you want me to|cannot .{0,40}from this session|the work must run in"
}

/// Append the universal debits (gate-block proxy via Bash is N/A — gate blocks are
/// scored from replay severity in reward.rs, not a pattern; deferral IS a pattern).
fn with_universal(mut rules: Vec<SignalRule>) -> Vec<SignalRule> {
    let universal = [
        rule(
            "deferral_handoff",
            EventClass::Stop,
            deferral_pattern(),
            DEFERRAL_WEIGHT,
        ),
        // Phase-4 enriched signals — stack-independent, matched on Write bodies.
        rule(
            "stub_shipped",
            EventClass::Write,
            r"(?i)\btodo!\s*\(|\bunimplemented!\s*\(|\bTODO:|\bFIXME\b|not implemented",
            STUB_SHIPPED_WEIGHT,
        ),
        rule(
            "silent_failure",
            EventClass::Write,
            r"unwrap_or_default\(\)|\.ok\(\);|except\s*:\s*pass|catch\s*\([^)]*\)\s*\{\s*\}",
            SILENT_FAILURE_WEIGHT,
        ),
        rule(
            "rca_present",
            EventClass::Write,
            r"(?i)\[RCA\]|ROOT CAUSE|5-why|root-cause",
            RCA_PRESENT_WEIGHT,
        ),
    ];
    rules.extend(universal.into_iter().flatten());
    rules
}

/// Rust + cargo (the default). Reproduces the original weights: build +10, test
/// +4, file-landed +1, substantive-test +4, deferral -12.
#[must_use]
pub fn rust_cargo() -> RewardRubric {
    let rules = with_universal(
        [
            rule(
                "build",
                EventClass::Bash,
                r"(?:^|&&|;|\|)\s*(?:[A-Z_][A-Z0-9_]*=\S+\s+)*cargo\s+(check|build)\b",
                10,
            ),
            rule(
                "test",
                EventClass::Bash,
                r"(?:^|&&|;|\|)\s*(?:[A-Z_][A-Z0-9_]*=\S+\s+)*cargo\s+(test|nextest)\b",
                4,
            ),
            rule("file", EventClass::Write, r".", 1),
            rule(
                "substantive_test",
                EventClass::Write,
                r"#\[test\]|#\[tokio::test\]",
                4,
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );
    let vacuous = Regex::new(
        r"assert!\s*\(\s*true\s*\)|assert_eq!\s*\(\s*(?:true\s*,\s*true|1\s*,\s*1|0\s*,\s*0)\s*\)|#\[test\]\s*(?:async\s+)?fn\s+\w+\s*\(\s*\)\s*\{\s*\}",
    )
    .ok();
    RewardRubric::new(rules, vacuous)
}

/// TypeScript + Bun (frontend / Workers). bun test / vitest / tsc / biome / playwright.
#[must_use]
pub fn ts_bun() -> RewardRubric {
    let rules = with_universal(
        [
            rule(
                "build",
                EventClass::Bash,
                r"(?:^|&&|;|\|)\s*(?:bun\s+run\s+)?(tsc|biome\s+check|bun\s+build)\b",
                10,
            ),
            rule(
                "test",
                EventClass::Bash,
                r"(?:^|&&|;|\|)\s*(?:bun\s+(test|run\s+test)|vitest|playwright\s+test)\b",
                4,
            ),
            rule("file", EventClass::Write, r".", 1),
            rule(
                "substantive_test",
                EventClass::Write,
                r"\b(it|test|describe)\s*\(",
                4,
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );
    let vacuous = Regex::new(r"expect\s*\(\s*true\s*\)\s*\.toBe\s*\(\s*true\s*\)|it\.skip\s*\(|it\s*\(\s*['\x22][^'\x22]*['\x22]\s*,\s*\(\s*\)\s*=>\s*\{\s*\}\s*\)").ok();
    RewardRubric::new(rules, vacuous)
}

/// Python + uv. pytest / ruff / uv run / mypy.
#[must_use]
pub fn python_uv() -> RewardRubric {
    let rules = with_universal(
        [
            rule(
                "build",
                EventClass::Bash,
                r"(?:^|&&|;|\|)\s*(?:uv\s+run\s+)?(ruff\s+check|mypy)\b",
                10,
            ),
            rule(
                "test",
                EventClass::Bash,
                r"(?:^|&&|;|\|)\s*(?:uv\s+run\s+)?pytest\b",
                4,
            ),
            rule("file", EventClass::Write, r".", 1),
            rule(
                "substantive_test",
                EventClass::Write,
                r"\bdef\s+test_\w+\s*\(",
                4,
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );
    let vacuous =
        Regex::new(r"assert\s+True\b|def\s+test_\w+\s*\([^)]*\)\s*:\s*(?:pass|\.\.\.)").ok();
    RewardRubric::new(rules, vacuous)
}

/// Resolve a named stack preset; unknown / absent → the Rust default.
#[must_use]
pub fn by_name(name: &str) -> RewardRubric {
    match name {
        "ts-bun" => ts_bun(),
        "python-uv" => python_uv(),
        _ => rust_cargo(),
    }
}
