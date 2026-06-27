use super::extract_research_topic;

#[test]
fn real_subject_becomes_topic() {
    assert_eq!(
        extract_research_topic("axum 0.8 middleware ordering", "implement"),
        "axum 0.8 middleware ordering"
    );
}

#[test]
fn instruction_filler_yields_no_topic() {
    // A meta/instruction sentence opening with stop-words is NOT a research subject —
    // capturing its first words wrongly blocks every production write that turn.
    for p in [
        "As the Fanout Model work in different Session Environment",
        "You have to fanout the cheap tier model",
        "Here also it is throwing the same messages",
        "this and then watch the build",
    ] {
        assert_eq!(
            extract_research_topic(p, "implement"),
            "",
            "instruction filler must not become a research topic: {p}"
        );
    }
}

#[test]
fn empty_prompt_is_no_topic() {
    assert_eq!(extract_research_topic("", "implement"), "");
}
