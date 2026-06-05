//! Stage 0: Root-Cause Analysis hard gate (CLAUDE.md §4). Blocks Edit/Write for
//! {debug,refactor,implement} at medium+ risk without an `[RCA]` block.
use crate::gates::pre_write_context::WriteContext;
use kavach_types::HookInput;

/// Scan the transcript for a same-turn `[RCA]` block, latch it into session
/// state, then enforce the RCA presence gate. `Some(reason)` blocks the write.
///
/// `input.last_assistant_message` is ALWAYS "" on `PreToolUse` (Claude Code only
/// populates it on Stop/SubagentStop), so RCA presence is carried purely by the
/// transcript scan + session state — never by an in-payload assistant message.
pub(super) fn rca_check(
    ctx: &WriteContext<'_>,
    input: &HookInput,
    session: &mut kavach_session::SessionState,
) -> Option<String> {
    let transcript_rca =
        super::super::pre_write_rca_guard::scan_transcript_for_rca(&input.transcript_path);
    if !session.rca_satisfied() && transcript_rca {
        session.mark_rca_present();
    }
    super::super::pre_write_rca_guard::check(
        ctx.tool_name,
        &session.intent_type,
        &session.intent_risk,
        "",
        session.rca_satisfied() || transcript_rca,
        ctx.file_path,
    )
}
