// TDD red phase: pins the `caveman_inject` API in crates/kavach-hook/src/inject.rs (not yet
// wired). Scope boundary: the fire-and-forget durable-spool metric write is NOT asserted here
// (async replay is untestable at unit scope) — only that it never panics the gate on empty input.
use crate::inject::caveman_inject;

#[test]
fn caveman_inject_returns_compressed_text() {
    let input = "the gate is binding then retry";
    let output = caveman_inject(input);
    assert!(
        output.len() < input.len(),
        "expected compressed output shorter than input, got output.len()={} input.len()={}",
        output.len(),
        input.len()
    );
    assert!(
        !output.contains(" the ") && !output.contains(" is "),
        "expected grammar words ' the '/' is ' dropped, got: {output:?}"
    );
}

#[test]
fn caveman_inject_preserves_protected_tokens() {
    let input = "see `crates/x.rs:9` and https://a.io/b then [RCA] applies";
    let output = caveman_inject(input);
    assert!(
        output.contains("`crates/x.rs:9`"),
        "expected file:line token preserved byte-for-byte, got: {output:?}"
    );
    assert!(
        output.contains("https://a.io/b"),
        "expected URL preserved byte-for-byte, got: {output:?}"
    );
    assert!(
        output.contains("[RCA]"),
        "expected [RCA] marker preserved byte-for-byte, got: {output:?}"
    );
}

#[test]
fn caveman_inject_never_panics_on_empty() {
    let output = caveman_inject("");
    assert!(
        output.is_empty(),
        "expected empty input to yield empty output, got: {output:?}"
    );
}

#[test]
fn caveman_inject_is_idempotent_on_preserved_only() {
    let input = "`code`";
    let output = caveman_inject(input);
    assert_eq!(
        output, input,
        "expected preserved-only input unchanged, got: {output:?}"
    );
}
