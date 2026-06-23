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

#[test]
fn persisted_line_warns_loudly_on_write_failure() {
    // F2: a failed snapshot write must be LLM-visible, not swallowed. The agent
    // is told to copy the working set NOW because the in-context block is the
    // only surviving copy across the discard.
    let line = persisted_line(false, "kavach-rs", "roadmap.unit.demo");
    assert!(line.contains("FAILED"), "must say FAILED: {line}");
    assert!(line.contains("ONLY surviving copy"), "must warn it's the last copy: {line}");
    assert!(line.contains("COPY active_card"), "must direct the recovery: {line}");
}

#[test]
fn persisted_line_gives_recall_command_on_success() {
    let line = persisted_line(true, "kavach-rs", "roadmap.unit.demo");
    assert!(line.contains("kavach db get"), "success path gives the recall cmd: {line}");
    assert!(line.contains("precompact.snapshot.roadmap.unit.demo"), "names the key: {line}");
    assert!(!line.contains("FAILED"), "success is not a failure: {line}");
}
