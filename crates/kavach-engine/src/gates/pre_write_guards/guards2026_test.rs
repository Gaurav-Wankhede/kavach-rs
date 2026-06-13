//! Engine-entry regression for the 2026 guard runner. Drives `check` (not the leaf
//! detectors in isolation) so the wiring — §DEDUP routed through the runner ahead of
//! `webhook`, blocking on a P0 — is proven at the integration boundary the engine
//! CLAUDE.md RULE requires for any new gate.
use super::check;
use super::super::result::Acc;
use crate::gates::pre_write_context::WriteContext;

/// Build a non-test, governed `WriteContext` for a Write of `content` to `path`.
fn ctx<'a>(path: &'a str, content: &'a str) -> WriteContext<'a> {
    WriteContext {
        file_path: path,
        tool_name: "Write",
        content,
        effective_content: content.to_owned(),
        is_code: true,
        is_test: false,
        is_rust: true,
        is_frontend: false,
    }
}

#[test]
fn dedup_redefinition_blocks_through_runner() {
    let src = "use core_utils::AppConfig;\npub struct AppConfig { url: String }\n";
    let mut acc = Acc::default();
    let block = check(&ctx("/x/crates/core/billing/src/cfg.rs", src), &mut acc);
    let reason = block.expect("redefining an imported object must block via the runner");
    assert!(reason.contains("DEDUP_P0"), "block reason tags the dedup gate: {reason}");
}

#[test]
fn clean_governed_file_passes_runner() {
    let src = "use core_utils::AppConfig;\nfn build(cfg: AppConfig) {}\n";
    let mut acc = Acc::default();
    assert!(
        check(&ctx("/x/crates/core/billing/src/cfg.rs", src), &mut acc).is_none(),
        "recalling the import (no redefinition) must not block"
    );
}
