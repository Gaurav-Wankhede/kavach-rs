//! Algorithm-Hunter + Architecture guards. Each requires a satisfying signal
//! (skill invoked OR `// ALGO:` / `// ARCH:` comment present) and may `Block`,
//! `AutoInject` context, or `Allow`. Block reasons short-circuit the chain.
use super::result::Acc;
use crate::gates::pre_write_context::WriteContext;
use kavach_session::SessionState;

/// True if `marker` is in the edit fragment, or (on an Edit) already on disk.
fn has_marker(ctx: &WriteContext<'_>, marker: &str) -> bool {
    let diff_has = ctx.content.contains(marker);
    let file_has = !diff_has
        && ctx.tool_name == "Edit"
        && std::fs::read_to_string(ctx.file_path).is_ok_and(|s| s.contains(marker));
    diff_has || file_has
}

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
    let satisfied = session.algo_hunter_invoked || has_marker(ctx, "// ALGO:");
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
    let satisfied = session.arch_skill_invoked || has_marker(ctx, "// ARCH:");
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
