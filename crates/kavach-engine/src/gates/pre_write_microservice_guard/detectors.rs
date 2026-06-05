//! Content detectors for the microservice-structure rules. Each honors the
//! `// hub:` / `// split:` escape-hatch markers where the original rule did.
use super::predicates::{
    HANDLER_FILE_LINE_LIMIT, MIXED_CONCERNS_LINE_LIMIT, MOD_RS_LINE_LIMIT, is_mod_rs,
    is_orchestrator,
};

pub(super) fn has_fn_body_in_mod(content: &str) -> bool {
    let lc = content.to_lowercase();
    // Allow files with an explicit hub marker — dispatch() in cmd/mod.rs is intentional.
    if lc.contains("// hub:") {
        return false;
    }
    (lc.contains("pub async fn ") || lc.contains("async fn ") || lc.contains("pub fn "))
        && lc.contains('{')
}

pub(super) fn has_struct_or_impl_in_mod(content: &str) -> bool {
    let lc = content.to_lowercase();
    if lc.contains("// hub:") {
        return false;
    }
    lc.contains("pub struct ") || lc.contains("struct ") || lc.contains("impl ")
}

pub(super) fn exceeds_mod_line_limit(content: &str) -> bool {
    content.lines().count() > MOD_RS_LINE_LIMIT
}

/// True when a non-mod, non-orchestrator file mixes struct definitions, impl
/// blocks, AND async handler functions — the classic mixed-concerns anti-pattern.
/// Files under `MIXED_CONCERNS_LINE_LIMIT` may still be cohesive; over it they
/// are not. Escape hatch: add `// split:` anywhere in the file to suppress.
pub(super) fn is_mixed_concerns_violation(file_path: &str, content: &str) -> bool {
    if is_mod_rs(file_path) || is_orchestrator(file_path) {
        return false;
    }
    if content.lines().count() <= MIXED_CONCERNS_LINE_LIMIT {
        return false;
    }
    let lc = content.to_lowercase();
    if lc.contains("// split:") {
        return false;
    }
    let has_struct = lc.contains("pub struct ") || lc.contains("struct ");
    let has_impl = lc.contains("impl ");
    let has_async = lc.contains("pub async fn ") || lc.contains("async fn ");
    has_struct && has_impl && has_async
}

/// Universal handler oversized-file detector: ANY `.rs` file (not mod.rs, not
/// orchestrator, not test) over 100 lines with 2+ `async fn` = colocated
/// handlers. Escape hatch: add `// split:` anywhere in the file to suppress.
pub(super) fn is_handler_oversized(file_path: &str, content: &str) -> bool {
    if is_mod_rs(file_path) || is_orchestrator(file_path) {
        return false;
    }
    if content.lines().count() <= HANDLER_FILE_LINE_LIMIT {
        return false;
    }
    let lc = content.to_lowercase();
    if lc.contains("// split:") {
        return false;
    }
    lc.matches("async fn ").count() > 1
}

pub(super) fn has_state_struct_in_orchestrator(content: &str) -> bool {
    let lc = content.to_lowercase();
    lc.contains("pub struct")
        && lc.contains("state")
        && (lc.contains("arc<") || lc.contains("pool") || lc.contains("pub fn new"))
}

/// True when service-init patterns (`::from_env`/`::from_config`/`::init` +
/// `Router::new()` + `.route(`) appear within a 500-char window, signalling
/// inline service init being added. Scattered patterns in large files don't fire.
pub(super) fn has_inline_service_init(content: &str) -> bool {
    let lc = content.to_lowercase();
    let has_init =
        lc.contains("::from_env(") || lc.contains("::from_config(") || lc.contains("::init(");
    if !(has_init && lc.contains("router::new()") && lc.contains(".route(")) {
        return false;
    }
    for init_pat in ["::from_env(", "::from_config(", "::init("] {
        if let Some(init_pos) = lc.find(init_pat) {
            let window_start = init_pos.saturating_sub(250);
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "bounded addition: init_pos + 500 is safely bounded by subsequent .min(lc.len())"
            )]
            let window_end = (init_pos + 500).min(lc.len());
            if let Some(window) = lc.get(window_start..window_end)
                && window.contains("router::new()")
                && window.contains(".route(")
            {
                return true;
            }
        }
    }
    false
}
