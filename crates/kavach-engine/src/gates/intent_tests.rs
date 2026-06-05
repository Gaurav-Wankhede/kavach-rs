use super::*;

#[test]
fn test_empty_prompt_handled() {
    let input = HookInput::default();
    run(&input).expect("run on empty prompt should not fail");
}

#[test]
fn test_extract_research_topic_short() {
    let topic = extract_research_topic("fix the bug", "debug");
    assert_eq!(topic, "fix the bug");
}

#[test]
fn test_extract_research_topic_long() {
    let prompt = "implement a new authentication system using JWT tokens with refresh capability";
    let topic = extract_research_topic(prompt, "implement");
    // Function takes first 6 words and joins them with spaces.
    assert_eq!(topic, "implement a new authentication system using");
}
