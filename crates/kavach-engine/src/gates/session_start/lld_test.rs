use super::lld_context;

#[test]
fn fires_for_kavach_project() {
    let out = lld_context("kavach-rs").expect("kavach project must get the LLD block");
    assert!(
        out.contains("[KAVACH_LLD]"),
        "carries the awareness tag: {out}"
    );
    assert!(
        out.contains("```mermaid"),
        "carries a renderable Mermaid diagram: {out}"
    );
    assert!(out.contains("kavach-engine"), "names the core crate: {out}");
}

#[test]
fn fires_for_kavach_prefixed_project() {
    assert!(lld_context("kavach").is_some(), "bare kavach slug");
    assert!(lld_context("kavach-web").is_some(), "kavach- prefixed slug");
}

#[test]
fn silent_for_non_kavach_project() {
    // The block is kavach-only: every other codebase's SessionStart is untouched,
    // so this never spends a foreign project's token budget on kavach's self-map.
    assert!(lld_context("iron-will").is_none());
    assert!(lld_context("some-other-app").is_none());
}

#[test]
fn silent_for_empty_slug() {
    assert!(lld_context("").is_none());
}
