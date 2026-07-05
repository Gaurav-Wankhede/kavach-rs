use super::vendor_is_exempt;
use kavach_hook::Vendor;

#[test]
fn claude_code_is_governed_not_exempt() {
    // The canonical/default vendor MUST run the laziness gate — incl. the
    // unset thread-local case, which resolves to ClaudeCode.
    assert!(!vendor_is_exempt(Vendor::ClaudeCode));
}

#[test]
fn every_other_vendor_is_exempt() {
    // Cursor (Composer 2.5) is the motivating case; the exemption applies to
    // every non-Claude-Code harness that spawns this binary.
    for v in [
        Vendor::Cursor,
        Vendor::Codex,
        Vendor::Antigravity,
        Vendor::Pi,
    ] {
        assert!(vendor_is_exempt(v), "{} must be exempt", v.name());
    }
}
