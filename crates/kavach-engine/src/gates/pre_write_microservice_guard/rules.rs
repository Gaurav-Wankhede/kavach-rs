//! P0 violation collectors: each returns the `(code, message)` tuples for one
//! file category, in the original dispatch order. Consumed by `check`.
use super::detectors::{
    exceeds_mod_line_limit, has_fn_body_in_mod, has_inline_service_init,
    has_state_struct_in_orchestrator, has_struct_or_impl_in_mod, is_handler_oversized,
    is_mixed_concerns_violation,
};

/// mod.rs hard-block rules: logic, structs/impls, and oversized hub.
pub(super) fn mod_rs(content: &str, out: &mut Vec<(&'static str, &'static str)>) {
    if has_fn_body_in_mod(content) {
        out.push((
            "LOGIC_IN_MOD",
            "mod.rs is a pure routing hub. Move fn bodies to dedicated files \
             (handler.rs, routes.rs, service.rs). mod.rs must only contain: \
             mod declarations, pub use re-exports, use statements.",
        ));
    }
    if has_struct_or_impl_in_mod(content) {
        out.push((
            "STRUCT_IN_MOD",
            "mod.rs must not define structs or impls. Move to state.rs, types.rs, \
             or the relevant domain file. Add '// hub:' comment to suppress if intentional.",
        ));
    }
    if exceeds_mod_line_limit(content) && !content.to_lowercase().contains("// hub:") {
        out.push((
            "MOD_TOO_LARGE",
            "mod.rs exceeds 100 lines. A hub file should only route — split logic \
             into focused files (handler.rs, routes.rs, service.rs, types.rs). \
             Add `// hub:` comment to suppress if this is an intentional dispatch hub.",
        ));
    }
}

/// Non-mod, non-orchestrator file rules: mixed-concerns + handler monolith.
pub(super) fn file(file_path: &str, content: &str, out: &mut Vec<(&'static str, &'static str)>) {
    if is_mixed_concerns_violation(file_path, content) {
        out.push((
            "FILE_MIXED_CONCERNS",
            "File mixes struct definitions, impl blocks, and async handlers over 200 lines. \
             Split into: types.rs (structs), service.rs (impl logic), handler.rs (async fn). \
             Add `// split:` comment to suppress if intentional.",
        ));
    }
    if is_handler_oversized(file_path, content) {
        out.push((
            "HANDLER_MONOLITH",
            "Handler/middleware file exceeds 100 lines with multiple async fn. \
             Rule: one async fn per file. Split each handler into its own file \
             (e.g. auth_middleware.rs, request_context_middleware.rs). \
             Add `// split:` comment to suppress if intentional.",
        ));
    }
}

/// Orchestrator rules: state structs + inline service init. The `// split:`
/// escape hatch lets a type-module lib.rs (e.g. kavach-types) opt out.
pub(super) fn orchestrator(content: &str, out: &mut Vec<(&'static str, &'static str)>) {
    if content.to_lowercase().contains("// split:") {
        return;
    }
    if has_state_struct_in_orchestrator(content) {
        out.push((
            "STATE_IN_ORCHESTRATOR",
            "Service state structs belong in the service module (e.g. media/state.rs), \
             not in app.rs/main.rs/lib.rs. Each service self-contains its initialization.",
        ));
    }
    if has_inline_service_init(content) {
        out.push((
            "INIT_IN_ORCHESTRATOR",
            "Service initialization (::from_env, ::from_config, ::init) belongs in the \
             service module's routes() fn. Use: service::routes(deps).",
        ));
    }
}
