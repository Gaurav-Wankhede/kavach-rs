//! `§CENTRALIZED_CONFIG` arm: a raw `env::var(...)` in `crates/{core,api,services}`
//! outside a config fragment / `main.rs` / dotenvy loader / startup validator —
//! env must be read once at boot via a typed fragment, not lazily per-request.
//! Path-sensitive (mirrors the `process::exit`-in-`main.rs` precedent); tests are
//! exempted globally by `detect()`.
//! SOURCE: .claude/rules/centralized-config.md · kavach `arch.centralized-config-LAW`.
use crate::severity::{Severity, Violation};

/// True when `path` is one of the enumerated `§CENTRALIZED_CONFIG` exceptions where
/// a raw env read is sanctioned: the fragment internals, the dotenvy loader,
/// `main.rs` boot wiring, or a startup env validator.
fn is_exempt_path(path: &str) -> bool {
    path.contains("/config_fragments/")
        || path.ends_with("/config.rs") && path.contains("/utils/")  // dotenvy loader
        || path.ends_with("/main.rs")
        || path.contains("/startup/env_validation")
        || path.contains("/startup/")
}

/// Only `crates/{core,api,services}` are governed; the harness, frontend, tools,
/// and tests are out of scope for this LAW.
fn is_governed_path(path: &str) -> bool {
    path.contains("/crates/core/")
        || path.contains("/crates/api/")
        || path.contains("/crates/services/")
}

/// Push a `P0Block` for each raw `env::var` read on a governed, non-exempt path.
///
/// Hard-block (not advisory): routed through `guards2026::severity::centralized_config`,
/// which returns the block reason from the pre-write chain. `is_exempt_path` excludes
/// every sanctioned env reader (fragment, dotenvy loader, `main.rs`, startup
/// validator), so a hit is a genuine LAW violation — `env_var_test.rs` proves the
/// false-positive set is empty. Promotion authorized by the user.
// O(1) extra space. The LAW bans one fixed call shape, so a two-needle substring
// test per line is optimal; a regex/Aho-Corasick automaton would add build cost
// with no gain for a literal match. Mirrors every other rust_guard leaf scanner.
pub(super) fn scan(file_path: &str, content: &str, v: &mut Vec<Violation>) {
    if !is_governed_path(file_path) || is_exempt_path(file_path) {
        return;
    }
    for (i, line) in content.lines().enumerate() {
        // Match `std::env::var(` or a bare `env::var(` call. Cheap substring
        // test — no regex needed; the LAW bans the literal call shape.
        let has_call = line.contains("std::env::var(") || line.contains("env::var(");
        if has_call {
            v.push(Violation::new(
                Severity::P0Block,
                "raw env::var (§CENTRALIZED_CONFIG)",
                "Read env via a typed config fragment (core_utils::XConfig::from_env()) — \
                 never raw env::var on a request/handler path. Add the var to a \
                 config_fragments/<provider>.rs fragment. See .claude/rules/centralized-config.md",
                i.saturating_add(1),
            ));
        }
    }
}

#[cfg(test)]
#[path = "env_var_test.rs"]
#[cfg(test)]
#[path = "env_var_test.rs"]
mod tests;