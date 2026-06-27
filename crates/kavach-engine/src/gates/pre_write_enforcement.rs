//! Stage 2: Enforcement checks — immutable migration, RCA, memory-query, skills,
//! research, evidence-chain, new-crate. Each sub-stage lives in a sibling module
//! and returns `Some(block_reason)`; the hub chains them so the first hit blocks.
mod gates;
mod package;
mod patterns;
mod rca;
#[cfg(test)]
#[path = "pre_write_enforcement_test.rs"]
#[cfg(test)]
#[path = "pre_write_enforcement_test.rs"]
mod tests;
use crate::gates::pre_write_context::WriteContext;
use kavach_types::HookInput;
/// Run all enforcement checks. Returns `Some(block_reason)` on hard block, None on pass.
pub(crate) fn check(
    ctx: &WriteContext<'_>,
    input: &HookInput,
    session: &mut kavach_session::SessionState,
) -> Option<String> {
    // -1. Immutable migration ledger — fires first: mutating an applied migration
    // is irreversible prod state-drift no RCA can undo. SOURCE: rca.immutable_migration_gate.
    if let Some(reason) = super::pre_write_immutable_migration::check(ctx.file_path) {
        return Some(reason);
    }
    rca::rca_check(ctx, input, session)
        .or_else(|| gates::memory_check(ctx, session))
        .or_else(|| gates::skill_check(ctx, session))
        .or_else(|| gates::research_check(ctx, input, session))
        .or_else(|| package::package_check(ctx, session))
}
