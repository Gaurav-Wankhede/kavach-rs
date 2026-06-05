//! File-classification predicates: extension, mod.rs, orchestrator, line limits.

pub(super) const MOD_RS_LINE_LIMIT: usize = 100;
/// Files over this threshold mixing struct+impl+async fn are mixed-concerns violations.
pub(super) const MIXED_CONCERNS_LINE_LIMIT: usize = 200;
pub(super) const HANDLER_FILE_LINE_LIMIT: usize = 100;

pub(super) fn is_orchestrator(path: &str) -> bool {
    let lc = path.to_lowercase();
    lc.ends_with("app.rs") || lc.ends_with("main.rs") || lc.ends_with("lib.rs")
}

/// Check if file has `.rs` extension (case-insensitive).
pub(super) fn is_rs_file(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
}

pub(super) fn is_mod_rs(path: &str) -> bool {
    path.to_lowercase().ends_with("mod.rs")
}
