//! Microservice Architecture Guard — enforces modular service structure.
//! Each service module must self-contain state, handlers, and types.
//! No dumping initialization logic into app.rs or lib.rs orchestrators.
//! mod.rs is a pure routing hub: no fn bodies, no struct/impl, max 100 lines.
//! Universal: applies to any Rust backend regardless of project layout.
//!
//! hub: re-exports `check` (P0 block) + `format_advisory` (P1); predicates,
//! detectors, and the per-category rule collectors live in submodules.
mod detectors;
mod predicates;
mod rules;

#[cfg(test)]
mod tests;

use super::platform_guard_msg::{build_advisory, build_block};
use super::platform_guard_paths::is_test;
use predicates::{is_mod_rs, is_orchestrator, is_rs_file};

/// P0 microservice-structure check. Returns a block message on any violation.
pub(crate) fn check(file_path: &str, content: &str) -> Option<String> {
    if !is_rs_file(file_path) || is_test(file_path) {
        return None;
    }
    let mut p0: Vec<(&str, &str)> = Vec::new();
    if is_mod_rs(file_path) {
        rules::mod_rs(content, &mut p0);
    }
    rules::file(file_path, content, &mut p0);
    if is_orchestrator(file_path) {
        rules::orchestrator(content, &mut p0);
    }
    if p0.is_empty() {
        return None;
    }
    Some(build_block("MICROSERVICE_GUARD", &p0))
}

/// P1 advisory for a service file mixing struct + impl + async handler + axum.
pub(crate) fn format_advisory(file_path: &str, content: &str) -> Option<String> {
    if !is_rs_file(file_path) || is_test(file_path) {
        return None;
    }
    // mod.rs + orchestrator violations are P0 hard blocks in check() — no advisory.
    if is_mod_rs(file_path) || is_orchestrator(file_path) {
        return None;
    }
    let lc = content.to_lowercase();
    let mut p1: Vec<(&str, &str)> = Vec::new();
    if lc.contains("pub struct")
        && lc.contains("pub async fn")
        && lc.contains("impl")
        && (lc.contains("axum::") || lc.contains("router"))
    {
        p1.push((
            "MIXED_CONCERNS",
            "Split service: state.rs (struct), handler.rs (async fn), types.rs \
             (request/response). One concern per file.",
        ));
    }
    if p1.is_empty() {
        return None;
    }
    Some(build_advisory("MICROSERVICE_GUARD", &p1))
}
