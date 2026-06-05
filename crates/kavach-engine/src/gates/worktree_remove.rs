use kavach_types::HookInput;

/// `WorktreeRemove` gate: clean up worktree state.
pub(crate) fn run(input: &HookInput) {
    let worktree_path = &input.worktree_path;

    if worktree_path.is_empty() {
        drop(kavach_hook::exit_silent());
    } else {
        let context = kavach_hook::context_block("WORKTREE_REMOVE", &[("path", worktree_path)]);
        drop(kavach_hook::exit_notification_context(&context));
    }
}
