//! Guard (P0, filesystem-collision): the FOREIGN-DIRTY-TREE guard (Case B of the
//! user-focus fix, operator directive 2026-06-18).
//!
//! ROOT CAUSE: the session-occupancy LEASE isolates CARDS, not the FILESYSTEM. Two
//! editing sessions sharing ONE git checkout collide at the file level — session B's
//! edits land on A's uncommitted work. The lease can't see this (different layer).
//! OBSERVED: the dispatcher handed this session a card while `git status` showed
//! 100+ dirty files from ANOTHER live session; editing would have clobbered it.
//!
//! THE GUARD: before the `AUTO_CONTINUE` dispatch chain, read `git status`. If the
//! tree is dirty FAR BEYOND this session's own writes (`session.files_modified`),
//! another live session is mid-edit on the shared checkout -> ALLOW the stop
//! (`exit_stop_context`, a coordinate-nudge) instead of dispatching an editing card
//! the agent would clobber. §safety > loop: a destructive file collision is a STOP.
//!
//! NON-TRIGGER: a clean tree, or a tree dirtied only by THIS session's own writes,
//! falls through -> the loop dispatches as normal (the autonomous value is intact).
//! Each session running its own `git worktree` makes this guard a permanent no-op
//! (zero foreign dirt) — the recommended N-session topology.

use core::ops::ControlFlow;

use super::super::shared::StopCtx;
use crate::gates::stop::foreign_tree_logic::foreign_dirty_count;

/// A tree dirty by more than this many files NOT attributable to this session is
/// treated as another session's live work-in-progress. Small slack absorbs
/// incidental churn (lockfiles, this session's own un-tracked writes).
const FOREIGN_DIRTY_THRESHOLD: usize = 8;

pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if ctx.input.stop_hook_active {
        return ControlFlow::Continue(());
    }
    // OPT-IN: guard concurrent editors via git-dirty heuristic (belt-and-suspenders).
    // See decision.engine.foreign-tree-opt-in.
    if std::env::var("KAVACH_FOREIGN_TREE_GUARD").as_deref() != Ok("1") {
        return ControlFlow::Continue(());
    }
    let Some(status) = git_status_porcelain() else {
        return ControlFlow::Continue(()); // not a git tree / git absent → no guard
    };
    let foreign = foreign_dirty_count(&status, &ctx.session.files_modified);
    if foreign < FOREIGN_DIRTY_THRESHOLD {
        return ControlFlow::Continue(());
    }
    crate::gates::event_log::log_gate_decision(
        &ctx.session.session_id,
        "stop:foreign_tree_guard",
        "allow_stop",
        &format!("foreign_dirty={foreign} — another session mid-edit on shared tree"),
        &ctx.session.project,
    );
    drop(kavach_hook::exit_stop_context(&format!(
        "[FOREIGN_TREE] git status shows {foreign} file(s) dirty that THIS session did \
         not write — another live session is mid-edit on this shared checkout. NOT \
         dispatching an editing card (it would clobber that session's uncommitted \
         work). COORDINATE: commit/stash the other session's changes, OR give each \
         editing session its own `git worktree add ../<proj>-<session>` (the N-session \
         topology). Re-stop once the tree reflects only this session's work."
    )));
    ControlFlow::Break(())
}

/// Impure: `git status --short -uno` stdout, or `None` when not a git tree / git
/// is unavailable (the guard then no-ops — fail-open on the diagnostic, never
/// blocking the loop on a missing tool).
fn git_status_porcelain() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["status", "--short", "-uno"])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}
// NOTE: this guard calls `exit_stop_context` (process exit) + shells `git`, so it
// is not unit-testable in-process. Its load-bearing logic is the PURE
// `foreign_tree_logic::foreign_dirty_count`, unit-tested in that module.
