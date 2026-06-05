//! Scope-narrowing advisory text. Single responsibility: the reusable primitive
//! the kept `kanban_status` dispatch guard uses to append a scope hint to its
//! re-dispatch message. The `block_and_record` mistake-then-HALT helper was
//! removed with the behavioral HALT guards under the "kill blocking, keep
//! auto-continue" policy.

/// Scope narrowing advisory to append to block messages. Empty on first block,
/// narrowing hint on second, tripped notice on third+.
pub(crate) fn get_scope_advisory(session: &kavach_session::SessionState, category: &str) -> String {
    session
        .scope_narrowing_hint(category)
        .map_or(String::new(), |hint| format!("\n\n{hint}"))
}
