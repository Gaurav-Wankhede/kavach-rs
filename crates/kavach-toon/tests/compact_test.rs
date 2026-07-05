use kavach_toon::compact::{Level, assert_lossless, compress};

#[test]
fn inline_code_span_survives_full_and_ultra() {
    let input = "run `useMemo` before the render, per the gate.";
    let full = compress(input, Level::Full);
    assert!(
        full.contains("`useMemo`"),
        "expected inline code span `useMemo` byte-for-byte in Full output, got: {full}"
    );
    let ultra = compress(input, Level::Ultra);
    assert!(
        ultra.contains("`useMemo`"),
        "expected inline code span `useMemo` byte-for-byte in Ultra output, got: {ultra}"
    );
}

#[test]
fn file_line_token_survives_byte_for_byte() {
    let input =
        "the root cause is documented at crates/kavach-hook/src/context.rs:94 in the ledger.";
    let full = compress(input, Level::Full);
    assert!(
        full.contains("crates/kavach-hook/src/context.rs:94"),
        "expected file:line token preserved byte-for-byte in Full output, got: {full}"
    );
    let ultra = compress(input, Level::Ultra);
    assert!(
        ultra.contains("crates/kavach-hook/src/context.rs:94"),
        "expected file:line token preserved byte-for-byte in Ultra output, got: {ultra}"
    );
}

#[test]
fn url_survives_byte_for_byte() {
    let input = "see the source at https://anthropic.com/engineering/x for the rationale.";
    let full = compress(input, Level::Full);
    assert!(
        full.contains("https://anthropic.com/engineering/x"),
        "expected URL preserved byte-for-byte in Full output, got: {full}"
    );
    let ultra = compress(input, Level::Ultra);
    assert!(
        ultra.contains("https://anthropic.com/engineering/x"),
        "expected URL preserved byte-for-byte in Ultra output, got: {ultra}"
    );
}

#[test]
fn bracket_signal_tokens_survive_byte_for_byte() {
    let input = "emit the [RCA] block before the fix, per [PRACTICE_DELTA] guidance.";
    let full = compress(input, Level::Full);
    assert!(
        full.contains("[RCA]"),
        "expected [RCA] preserved byte-for-byte in Full output, got: {full}"
    );
    assert!(
        full.contains("[PRACTICE_DELTA]"),
        "expected [PRACTICE_DELTA] preserved byte-for-byte in Full output, got: {full}"
    );
}

#[test]
fn version_number_survives_byte_for_byte() {
    let input = "the crate was bumped to 0.3.1 in the last release.";
    let full = compress(input, Level::Full);
    assert!(
        full.contains("0.3.1"),
        "expected version token 0.3.1 preserved byte-for-byte in Full output, got: {full}"
    );
    let ultra = compress(input, Level::Ultra);
    assert!(
        ultra.contains("0.3.1"),
        "expected version token 0.3.1 preserved byte-for-byte in Ultra output, got: {ultra}"
    );
}

#[test]
fn fenced_code_block_survives_untouched() {
    let input = "the diagram is below:\n```mermaid\ngraph TD; A-->B;\n```\nthat is the LLD.";
    let fence = "```mermaid\ngraph TD; A-->B;\n```";
    let full = compress(input, Level::Full);
    assert!(
        full.contains(fence),
        "expected fenced code block preserved untouched in Full output, got: {full}"
    );
}

#[test]
fn full_drops_droppable_article_and_copula() {
    let input = "the gate is binding on this turn.";
    let full = compress(input, Level::Full);
    assert!(
        !full.contains(" the "),
        "expected Full to drop the droppable article \"the\", got: {full}"
    );
    assert!(
        !full.contains(" is "),
        "expected Full to drop the droppable copula \"is\", got: {full}"
    );
}

#[test]
fn full_token_count_is_less_than_input_for_plain_prose() {
    let input = "the block that denies the tool call is a binding instruction, not an error to route around.";
    let full = compress(input, Level::Full);
    let input_tokens = input.split_whitespace().count();
    let full_tokens = full.split_whitespace().count();
    assert!(
        full_tokens < input_tokens,
        "expected Full token count ({full_tokens}) < input token count ({input_tokens})"
    );
}

#[test]
fn lite_is_at_least_as_long_as_ultra_for_same_prose() {
    let input = "a gate that denies or blocks a tool call is a binding instruction, not an error to route around.";
    let lite = compress(input, Level::Lite);
    let ultra = compress(input, Level::Ultra);
    assert!(
        lite.len() >= ultra.len(),
        "expected len(Lite)={} >= len(Ultra)={} for same prose input",
        lite.len(),
        ultra.len()
    );
}

#[test]
fn assert_lossless_ok_when_preserved_present_err_when_corrupted() {
    let original = "see https://anthropic.com/engineering/x for the rationale.";
    let compressed = "see https://anthropic.com/engineering/x rationale.";
    assert_lossless(original, compressed)
        .expect("expected Ok when preserved URL token is present in both strings");

    let corrupted = "see https://anthropic.com/engineerin/x rationale.";
    let result = assert_lossless(original, corrupted);
    assert!(
        result.is_err(),
        "expected Err when a preserved URL token is hand-mangled (dropped a char), got Ok"
    );
}
