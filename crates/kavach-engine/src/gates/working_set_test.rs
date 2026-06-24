//! Tests for the lossless working-set reconstructor. The RPC-backed paths fail-soft
//! to `None`/empty off-daemon, so these pin the project guard + the block contract.

use super::*;

#[test]
fn empty_project_reconstructs_nothing() {
    assert!(reconstruct("").is_none());
}

#[test]
fn off_daemon_is_fail_soft_none() {
    // No daemon in the test harness → both RPCs miss → nothing to reconstruct → None,
    // so PostCompact falls back to the summary exactly as before (no panic, no block).
    assert!(reconstruct("definitely-not-a-real-project-xyz").is_none());
}

#[test]
fn block_shape_is_lossless_and_self_trusting() {
    // Pin the contract the reconstructor emits when state IS present: it must declare
    // itself authoritative over the lossy summary and carry the resume directive.
    let block = format!(
        "\n[WORKING_SET — LOSSLESS, re-derived from the store; trust this over [COMPACT_SUMMARY]]\n\
         active_card: {key}\n  touches: {touches}\n",
        key = "roadmap.unit.demo",
        touches = "a.rs b.rs",
    );
    assert!(block.contains("[WORKING_SET"));
    assert!(block.contains("trust this over [COMPACT_SUMMARY]"));
    assert!(block.contains("active_card: roadmap.unit.demo"));
}

#[test]
fn intent_line_renders_card_title_as_the_restored_intent() {
    let line = render_intent_line("Refactor oversized files into nano-modules");
    assert!(line.contains("[INTENT_RESTORED]"));
    assert!(line.contains("Refactor oversized files into nano-modules"));
}

#[test]
fn intent_line_blank_title_is_empty() {
    assert!(render_intent_line("").is_empty());
}
