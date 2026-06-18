// PreToolUse consumer for §LSP-FIRST state.
//
// ARCH: LspFirstAdvisoryGate
// PATTERN: pre_write_advisory | SCOPE: file | CAP: AP | SEARCHED: 2026-05
// TIME: O(N) — N = files diagnosed (typically <10 per session)
// SPACE: O(1) per call
//
// SOURCE: ~/.claude/CLAUDE.md §LSP-FIRST — enforcement clause.
// SOURCE: crates/kavach-engine/CLAUDE.md — "default to P1Advisory unless
//   irreversible AND FP rate <1%". LSP-first is reversible (skip = grep).
// SOURCE: github.com/anthropics/claude-code#37210, #33106, #52822 —
//   permissionDecision-deny is unreliable; advisory-via-context is
//   complementary (the gate output gets injected via [P1_ADVISORIES] block
//   in pre_write.rs, which surfaces to the model on the next prompt).

/// Build a P1 advisory string when the edit target hasn't had its LSP
/// diagnostics observed this session. Returns None if either (a) the file
/// type has no installed LSP server (fall through to grep), or (b) the
/// file IS in `session.lsp_diag_seen`.
///
/// The advisory is injected via `pre_write.rs`'s [`P1_ADVISORIES`] block; it
/// does NOT hard-block the edit. The first iteration prioritizes signal
/// over enforcement — once FP rate is measured in real traffic the policy
/// can promote to P0 (per crates/kavach-engine/CLAUDE.md severity policy).
#[must_use]
pub(crate) fn advisory(file_path: &str, session: &kavach_session::SessionState) -> Option<String> {
    if file_path.is_empty() {
        return None;
    }
    if !is_lsp_supported(file_path) {
        return None;
    }
    if session.lsp_diag_seen.iter().any(|p| p == file_path) {
        return None;
    }
    Some(format!(
        "[LSP_FIRST_ADVISORY] §LSP-FIRST: No LSP diagnostics observed for \
         {file_path} this session. Per ~/.claude/CLAUDE.md §LSP-FIRST, an \
         LSP go-to-definition / find-references / hover / get_diagnostics \
         call SHOULD precede an Edit/Write on a code file. Skip with \
         [LSP_FALLBACK]<reason> if no server is available."
    ))
}

/// True for file extensions where an Anthropic-official LSP plugin is
/// known to exist (rust-analyzer-lsp, typescript-lsp, pyright, …). Files
/// outside this set get no LSP advisory (markdown, json, toml, sql, …
/// don't have meaningful LSP nav for our purposes).
/// SOURCE: github.com/Piebald-AI/claude-code-lsps — current marketplace.
fn is_lsp_supported(path: &str) -> bool {
    const LSP_SUPPORTED_EXTS: &[&str] = &[
        ".rs", // rust-analyzer-lsp
        ".ts", ".tsx", ".js", ".jsx", ".mts", ".cts", // typescript-lsp
        ".py",  // pyright
        ".go",  // gopls
        ".java", ".kt", ".kts", // java/kotlin
        ".scala", ".c", ".cc", ".cpp", ".h", ".hpp", // clangd
        ".cs",  // C#
        ".rb",  // solargraph
        ".vue", ".svelte", // framework-specific
        ".ml", ".mli", // OCaml
        ".dart", ".sol", // solidity
    ];
    LSP_SUPPORTED_EXTS.iter().any(|ext| path.ends_with(ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_exts_detect_known_languages() {
        assert!(is_lsp_supported("crates/foo/src/lib.rs"));
        assert!(is_lsp_supported("frontend/app.tsx"));
        assert!(is_lsp_supported("scripts/lint.py"));
        assert!(is_lsp_supported("services/api.go"));
        assert!(!is_lsp_supported("README.md"));
        assert!(!is_lsp_supported("Cargo.toml"));
        assert!(!is_lsp_supported("config.json"));
    }

    #[test]
    fn no_advisory_when_file_seen() {
        let mut s = kavach_session::SessionState::default();
        s.lsp_diag_seen.push("crates/foo/src/lib.rs".into());
        assert!(advisory("crates/foo/src/lib.rs", &s).is_none());
    }

    #[test]
    fn no_advisory_for_unsupported_file() {
        let s = kavach_session::SessionState::default();
        assert!(advisory("README.md", &s).is_none());
        assert!(advisory("Cargo.toml", &s).is_none());
    }

    #[test]
    fn advisory_emitted_for_unseen_supported_file() {
        let s = kavach_session::SessionState::default();
        let out = advisory("crates/foo/src/lib.rs", &s);
        assert!(out.is_some());
        let msg = out.unwrap();
        assert!(msg.contains("§LSP-FIRST"));
        assert!(msg.contains("crates/foo/src/lib.rs"));
    }

    #[test]
    fn empty_path_returns_none() {
        let s = kavach_session::SessionState::default();
        assert!(advisory("", &s).is_none());
    }
}
