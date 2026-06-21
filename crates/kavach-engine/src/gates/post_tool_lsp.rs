// LSP-FIRST state producer: record diagnosed files to session.
// See decision.engine.lsp_first_producer.

use kavach_types::HookInput;

/// Bound the on-disk `lsp_diag_seen` list. SOURCE: cold reviewer FIX-G —
/// large monorepos could accumulate 10K+ paths over a long session;
/// O(n) `iter()` lookup + serialize cost grows with it. Cap at 500: the
/// oldest entry rotates out FIFO when the cap is reached. The actual
/// per-turn working set is far smaller than this (5-50 files), so the
/// cap only matters for marathon sessions or monorepo-wide grep storms.
const LSP_DIAG_SEEN_CAP: usize = 500;

/// Producer-side handler: when an LSP tool fires, record the target file
/// into `session.lsp_diag_seen` so the consumer-side §LSP-FIRST gate on
/// Edit/Write/MultiEdit can confirm "diagnostics were read for this file
/// before the edit attempt".
pub(crate) fn handle(input: &HookInput, session: &mut kavach_session::SessionState) {
    if !is_lsp_tool(&input.tool_name) {
        return;
    }
    if let Some(path) = extract_target_path(input)
        && !session.lsp_diag_seen.iter().any(|p| p == &path)
    {
        session.lsp_diag_seen.push(path);
        // FIX-G: FIFO rotation at LSP_DIAG_SEEN_CAP — oldest entry drops
        // when we exceed the bound. Keeps the most-recent N files in
        // working memory; perfectly matches LRU semantics for the
        // §LSP-FIRST advisory's per-edit lookup.
        if session.lsp_diag_seen.len() > LSP_DIAG_SEEN_CAP {
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "drop_n is bounded: len > CAP implies len - CAP < len; usize underflow impossible"
            )]
            let drop_n = session.lsp_diag_seen.len() - LSP_DIAG_SEEN_CAP;
            session.lsp_diag_seen.drain(..drop_n);
        }
        session.save_or_log();
    }
}

/// True for any tool name that corresponds to an LSP operation. Covers:
///   • native Claude Code LSP tool — exact name `LSP`
///   • cclsp MCP bridge — prefix `mcp__cclsp__`
///   • Anthropic-official LSP plugins (rust-analyzer-lsp, typescript-lsp)
///     — prefix `mcp__lsp_` / `mcp__rust_analyzer` / `mcp__typescript_lsp`
fn is_lsp_tool(name: &str) -> bool {
    name == "LSP"
        || name.starts_with("mcp__cclsp__")
        || name.starts_with("mcp__lsp_")
        || name.starts_with("mcp__rust_analyzer")
        || name.starts_with("mcp__typescript_lsp")
}

/// Extract the file path the LSP call was scoped to. LSP MCP tools
/// commonly accept `file_path` (lsp-mcp schema convention); fall back to
/// `uri` for servers that use the raw LSP wire format. Returns None for
/// whole-workspace queries (e.g. `find_workspace_symbols`) — those don't
/// satisfy the per-file diagnostic-seen invariant.
fn extract_target_path(input: &HookInput) -> Option<String> {
    let ti = input.tool_input.as_ref()?;
    if let Some(p) = ti.get("file_path").and_then(|v| v.as_str()) {
        return Some(p.to_owned());
    }
    if let Some(u) = ti.get("uri").and_then(|v| v.as_str()) {
        return Some(u.trim_start_matches("file://").to_owned());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsp_tool_recognition() {
        assert!(is_lsp_tool("LSP"));
        assert!(is_lsp_tool("mcp__cclsp__find_references"));
        assert!(is_lsp_tool("mcp__lsp_go_to_definition"));
        assert!(is_lsp_tool("mcp__rust_analyzer__hover"));
        assert!(is_lsp_tool("mcp__typescript_lsp__rename"));
        assert!(!is_lsp_tool("WebSearch"));
        assert!(!is_lsp_tool("Bash"));
        assert!(!is_lsp_tool("Edit"));
        assert!(!is_lsp_tool("mcp__github__list_issues"));
    }
}
