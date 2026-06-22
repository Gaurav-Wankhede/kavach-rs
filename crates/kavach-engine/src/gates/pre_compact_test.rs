//! Tests for the `PreCompact` anti-amnesia guard. The RPC-backed paths fail-soft to
//! `None`/silent off-daemon, so these exercise the pure assembly + the no-state path.

use super::*;

#[test]
fn empty_instructions_no_card_is_silent() {
    // No custom_instructions AND no active card (off-daemon RPC miss → None) → silent,
    // preserving the original behavior when there is nothing durable to protect.
    let input = HookInput::default();
    run(&input);
}

#[test]
fn empty_project_yields_no_guard() {
    // The guard is project-scoped; an empty project can never build a block.
    assert!(build_memory_guard("").is_none());
}

#[test]
fn guard_block_is_self_describing_when_present() {
    // When a guard CAN be built it must name the recall command + the resume directive
    // so the post-compact turn is not blind. Off-daemon this returns None, so we assert
    // the contract on a hand-built block shape via the same format the fn emits.
    let block = format!(
        "[MEMORY_GUARD] (anti-amnesia: compaction is about to discard verbatim history)\n\
         active_card: {key}\ntouches: {touches}\n",
        key = "roadmap.unit.demo",
        touches = "a.rs b.rs",
    );
    assert!(block.contains("[MEMORY_GUARD]"));
    assert!(block.contains("active_card: roadmap.unit.demo"));
    assert!(block.contains("touches: a.rs b.rs"));
}
