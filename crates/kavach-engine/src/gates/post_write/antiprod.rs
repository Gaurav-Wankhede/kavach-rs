//! Stage 1: anti-production pattern scan. P0 (mock data) hard-blocks; lower
//! tiers push fix context. Returns true when the write was hard-blocked.

/// Run the antiprod scan. On a P0 hit, emit the block and return true (caller
/// must return early); otherwise push any fix context and return false.
pub(super) fn check_antiprod(
    file_path: &str,
    content: &str,
    context_parts: &mut Vec<String>,
) -> bool {
    if content.is_empty() || file_path.is_empty() {
        return false;
    }
    let violations = kavach_patterns::detect_antiprod(file_path, content);
    if violations.is_empty() {
        return false;
    }
    let fix_ctx = super::super::fix_instructions::generate_fix_instructions(&violations, file_path);
    let has_p0 = violations
        .iter()
        .any(|v| matches!(v.level, kavach_patterns::AntiProdLevel::P0MockData));
    if has_p0 {
        drop(kavach_hook::exit_post_tool_block(
            "P0 violation detected — fix required before continuing",
            &fix_ctx,
        ));
        return true;
    }
    context_parts.push(fix_ctx);
    false
}

#[cfg(test)]
mod tests {
    use super::check_antiprod;

    // Gate-boundary wiring invariant: the post_write gate MUST drive the
    // anti-prod scanner. Without this, a refactor that severs the
    // check_antiprod → detect_antiprod call would silently stop P0 mock-data
    // from blocking, with the whole suite still green (the FIXEDBENCH drift
    // mode that already bypassed the rule-engine path in this codebase).
    #[test]
    fn p0_mock_data_blocks_the_write() {
        let mut ctx = Vec::new();
        // `const mockUsers = [` is a known P0MockData trigger (kavach-patterns
        // detect_test.rs). A P0 hit must return true so the caller returns early.
        let blocked = check_antiprod("src/users.tsx", "const mockUsers = [];", &mut ctx);
        assert!(
            blocked,
            "P0 mock-data write must be hard-blocked by the gate"
        );
    }

    #[test]
    fn clean_content_passes_through() {
        let mut ctx = Vec::new();
        let blocked = check_antiprod(
            "src/add.rs",
            "pub fn add(a: i32, b: i32) -> i32 { a + b }",
            &mut ctx,
        );
        assert!(!blocked, "clean content must not be blocked");
    }

    #[test]
    fn empty_inputs_are_noops() {
        let mut ctx = Vec::new();
        assert!(!check_antiprod("", "x", &mut ctx));
        assert!(!check_antiprod("a.rs", "", &mut ctx));
    }
}
