use kavach_types::HookInput;

use crate::error::EngineError;

/// `WorktreeCreate` gate: track active worktrees in session.
#[expect(
    clippy::unnecessary_wraps,
    reason = "gate signature matches hook contract"
)]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let worktree_name = &input.name;

    if worktree_name.is_empty() {
        drop(kavach_hook::exit_silent());
    } else {
        let context = kavach_hook::context_block("WORKTREE_CREATE", &[("name", worktree_name)]);
        drop(kavach_hook::exit_notification_context(&context));
    }
    Ok(())
}
