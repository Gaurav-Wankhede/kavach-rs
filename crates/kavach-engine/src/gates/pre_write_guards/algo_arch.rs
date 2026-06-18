//! Algorithm-Hunter + Architecture guards. The ONLY satisfying signal is the
//! skill invocation (which records to a decision row) — never an inline
//! provenance comment. Each may `Block`, `AutoInject` context, or `Allow`.
use super::result::Acc;
use crate::gates::pre_write_context::WriteContext;
use kavach_session::SessionState;

/// Algorithm Hunter guard. Block reason short-circuits; `AutoInject` sets advisory.
pub(super) fn algo(
    ctx: &WriteContext<'_>,
    session: &SessionState,
    acc: &mut Acc,
) -> Option<String> {
    use super::super::pre_write_algo_guard::AlgoGuardOutcome::{Allow, AutoInject, Block};
    if !ctx.is_rust || ctx.is_test {
        return None;
    }
    let satisfied = session.algo_hunter_invoked;
    match super::super::pre_write_algo_guard::check(
        ctx.file_path,
        ctx.content,
        satisfied,
        &session.project,
    ) {
        Block(msg) => Some(msg),
        AutoInject(inject_ctx) => {
            acc.algo_advisory = Some(inject_ctx);
            None
        }
        Allow => None,
    }
}

/// Architecture guard — requires /arch skill or `// ARCH:` comment.
pub(super) fn arch(
    ctx: &WriteContext<'_>,
    session: &SessionState,
    acc: &mut Acc,
) -> Option<String> {
    use super::super::pre_write_arch_guard::ArchPreWriteOutcome::{Allow, AutoInject, Block};
    if !ctx.is_rust || ctx.is_test {
        return None;
    }
    let satisfied = session.arch_skill_invoked;
    match super::super::pre_write_arch_guard::check(
        ctx.file_path,
        ctx.content,
        satisfied,
        &session.project,
    ) {
        Block(msg) => Some(msg),
        AutoInject(inject_ctx) => {
            acc.merge_advisory(inject_ctx);
            None
        }
        Allow => None,
    }
}
