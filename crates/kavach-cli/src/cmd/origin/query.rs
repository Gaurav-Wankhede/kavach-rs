//! `kavach origin --query '<json>'` — the dynamic role-query path.

use super::role_query::RoleQuery;
use super::{scorer, walker};
use std::path::Path;

/// Exit: 0 = at least one candidate ≥ threshold, 1 = none, 2 = bad input/root.
#[must_use]
pub(super) fn run(json: &str, root: &Path) -> i32 {
    if !root.exists() {
        eprintln!("origin: target path missing: {}", root.display());
        return 2;
    }
    let q = match RoleQuery::parse(json) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("origin: {e}");
            return 2;
        }
    };
    let ranked = scorer::rank(&q, walker::walk(root));
    if ranked.is_empty() {
        println!("[KAVACH_ORIGIN] role '{}': no candidate >= {:.2} (broaden value_regex / env_key_hints, or try sg / rust-analyzer)", q.role, scorer::THRESHOLD);
        return 1;
    }
    let top = &ranked[0];
    let shape = if top.cand.is_secret { " [secret: location only]" } else { "" };
    println!(
        "[KAVACH_ORIGIN] role '{}' -> {} '{}' at {}:{} (score {:.2}){shape}",
        q.role,
        top.cand.kind.label(),
        top.cand.name,
        top.cand.file,
        top.cand.line,
        top.score
    );
    let extra = ranked.len().saturating_sub(1);
    if extra > 0 {
        println!("  +{extra} more candidate(s) — tighten the query");
    }
    0
}

#[cfg(test)]
#[path = "query_test.rs"]
mod query_test;
