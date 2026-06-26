//! Discard + race multiline arms (indices 45-58): magic literals, blanket error
//! mapping, silent Result/await discards, unused-param suppression, and the
//! file/option/map/db/counter data-race detectors.
//!
//! All arms fire on a single match, differing only in scope (whole-content vs
//! per-line), so they live in one `(idx, line_scoped, severity, pattern, fix)`
//! table — the canonical `too_many_lines` remedy.
//! SOURCE: <https://rust-lang.github.io/rust-clippy/master/index.html> (`too_many_lines`)
use crate::severity::{Severity, Violation};
use regex::Regex;

fn one(v: &mut Vec<Violation>, sev: Severity, pat: &str, fix: &str) {
    v.push(Violation::new(sev, pat, fix, 0));
}

/// `(regex index, line-scoped?, severity, pattern, fix)`. `line_scoped` arms test
/// each line individually (magic literals); the rest match against whole content.
const ROWS: &[(usize, bool, Severity, &str, &str)] = {
    use Severity::{P0Block, P1Advisory};
    &[
        (
            45,
            true,
            P1Advisory,
            "magic number",
            "Extract to a named constant — bare numbers in logic are unreadable",
        ),
        (
            46,
            false,
            P1Advisory,
            "long if/else chain",
            "Use enum + match instead of if/else if/else if — exhaustive matching catches missing cases",
        ),
        (
            47,
            true,
            P1Advisory,
            "magic string",
            "Extract to a named constant or use an enum variant — string comparisons are fragile",
        ),
        (
            48,
            false,
            P1Advisory,
            "blanket sqlx error conversion",
            "Match on sqlx::Error variants and PostgreSQL SQLSTATE codes (23505→409, 23503→422, RowNotFound→404). Invoke /error",
        ),
        (
            49,
            false,
            P1Advisory,
            "blanket 500 in error handler",
            "Match on error variants — RowNotFound→404, unique violation→409, FK→422, PoolTimeout→503. Only unknown errors are 500",
        ),
        (
            50,
            false,
            P0Block,
            "silent DB result discard",
            "Check the Result — use `?` to propagate or match to handle. `let _result =` hides failures",
        ),
        (
            51,
            false,
            P0Block,
            "silent HTTP result discard",
            "Check the Result — HTTP calls fail. `let _response =` silently drops errors",
        ),
        (
            52,
            false,
            P0Block,
            "silent await discard",
            "Check the Result of .await — use `?` or match. `let _ = x.await` hides async failures",
        ),
        (
            53,
            false,
            P0Block,
            "unused param suppression",
            "Use the parameter or remove it — `_name: &Type` hides debt. Run `kavach db write --category roadmap` if deferred",
        ),
        (
            54,
            false,
            P0Block,
            "file TOCTOU race",
            "Use File::open() directly and handle NotFound — exists() check creates race window (CWE-367)",
        ),
        (
            55,
            false,
            P0Block,
            "check-then-set race",
            "Use Option::get_or_insert() or atomic compare-and-swap — if is_none then set races",
        ),
        (
            56,
            false,
            P0Block,
            "get-then-insert race",
            "Use HashMap::entry().or_insert() — get() then insert() races under concurrency",
        ),
        (
            57,
            false,
            P0Block,
            "DB read-then-write race",
            "Wrap SELECT+UPDATE in transaction — concurrent requests cause lost updates (CWE-362)",
        ),
        (
            58,
            false,
            P1Advisory,
            "non-atomic increment",
            "Use AtomicU64::fetch_add() or Mutex — `x += 1` races under concurrent access",
        ),
        (
            79,
            false,
            P1Advisory,
            "anonymous let _ = discards a call or live binding",
            "`let _ = call()` swallows a Result/#[must_use]; `let _ = (a, b)` discards live values. ACT on it (`?`/match/if), or for a genuine unit/guard use `drop(x)` — never `let _ =` of a fallible call or live binding",
        ),
    ]
};

/// True when regex `idx` matches: whole-content, or any single line when `line_scoped`.
fn matches(regexes: &[Regex], idx: usize, line_scoped: bool, content: &str) -> bool {
    let Some(re) = regexes.get(idx) else {
        return false;
    };
    if line_scoped {
        content.lines().any(|l| re.is_match(l))
    } else {
        re.is_match(content)
    }
}

/// Compiled RAII / scope-guard allow floor — stems legitimately bound as
/// `let _name = …` (held to end-of-scope for their Drop, not a discarded signal).
/// The general discard arm (regex index 78) skips a match whose captured name
/// matches one of these (done per-match because this regex engine has no
/// lookaround). IMMUTABLE floor: the dynamic overlay can only ADD stems, never
/// remove one — a safety pattern must not be deletable via the DB.
const RAII_DISCARD_ALLOW: &[&str] = &["guard", "lock", "span", "permit", "g", "defer", "entered"];

/// The gate-config key whose `pattern_list` overlay ADDS extra RAII allow-stems.
/// Resolved under the global project — RAII naming is project-agnostic at the
/// pre-write layer, which has no project in scope.
const RAII_ALLOW_KEY: &str = "rust_guard.raii_discard_allow";

/// Resolve the effective RAII allow-list: the compiled floor plus any DB overlay
/// stems. The pattern crate is a leaf below the RPC client, so it injects a
/// miss-only `call` here — the floor is always honored, and the DB-extras path
/// activates the moment a transport-bearing caller supplies a real `call`. This
/// is the `unit.gate-cfg-patterns-safelist-wireup` adoption: the detector now
/// resolves through `kavach_types::gate_patterns` instead of a bare const.
fn effective_raii_allow() -> Vec<String> {
    // Leaf-crate transport seam: no RPC client below `kavach-rpc`, so resolve
    // with a miss closure. `gate_patterns` returns the compiled floor unchanged
    // on a miss (fail-closed); a transport-bearing variant can replace this.
    kavach_types::gate_patterns(
        |_: &str, _: &str| None,
        kavach_types::gate_config::GLOBAL_PROJECT_KEY,
        RAII_ALLOW_KEY,
        RAII_DISCARD_ALLOW,
    )
}

/// True when the captured `_name` is a RAII guard binding (prefix match after the
/// leading underscore, so `_guard`, `_guard2`, `_lockA` all pass). Matches against
/// the effective allow-list (compiled floor + DB overlay).
fn is_raii_name(name: &str) -> bool {
    let stem = name.trim_start_matches('_').to_ascii_lowercase();
    effective_raii_allow()
        .iter()
        .any(|a| stem == *a || stem.starts_with(a.as_str()))
}

/// Index-78 arm: general named-underscore discard. Fires once per non-RAII
/// `let _name = <expr>` line. Separate from the table because it captures the
/// binding name to filter RAII guards (the table only does `is_match`).
fn scan_named_discard(r: &[Regex], content: &str, v: &mut Vec<Violation>) {
    let Some(re) = r.get(78) else {
        return;
    };
    let mut fired = false;
    for caps in content.lines().filter_map(|l| re.captures(l)) {
        let Some(name) = caps.get(1).map(|m| m.as_str()) else {
            continue;
        };
        if is_raii_name(name) {
            continue;
        }
        fired = true;
        break;
    }
    if fired {
        one(
            v,
            Severity::P1Advisory,
            "let _name discards a return value",
            "`let _name = …` names a value then throws it away — a discarded signal. \
             If the return carries a decision (bool/Result/count), ACT on it (match/if/?, \
             propagate, or log on the failure arm). For true fire-and-forget use `let _ =` or `drop(…)`",
        );
    }
}

pub(super) fn scan(r: &[Regex], content: &str, v: &mut Vec<Violation>) {
    for &(idx, line_scoped, sev, pat, fix) in ROWS {
        if matches(r, idx, line_scoped, content) {
            one(v, sev, pat, fix);
        }
    }
    scan_named_discard(r, content, v);
}
