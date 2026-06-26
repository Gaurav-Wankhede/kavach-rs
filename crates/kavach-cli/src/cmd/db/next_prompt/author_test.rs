use super::authoring_prompt;

#[test]
fn embeds_all_card_fields() {
    let p = authoring_prompt("proj", "roadmap.unit.x", "Retire gRPC", "Swap to bindings.");
    assert!(p.contains("proj"));
    assert!(p.contains("roadmap.unit.x"));
    assert!(p.contains("Retire gRPC"));
    assert!(p.contains("Swap to bindings."));
}

#[test]
fn names_the_seven_blocks() {
    let p = authoring_prompt("p", "k", "t", "c");
    for block in [
        "ROLE",
        "TASK",
        "FILES",
        "CONSTRAINTS",
        "VERIFY",
        "DONE WHEN",
        "ON FAILURE",
    ] {
        assert!(p.contains(block), "missing block {block}");
    }
}

#[test]
fn forbids_preamble_so_output_is_servable_verbatim() {
    let p = authoring_prompt("p", "k", "t", "c");
    assert!(p.contains("ONLY"));
    assert!(p.contains("verbatim"));
}
