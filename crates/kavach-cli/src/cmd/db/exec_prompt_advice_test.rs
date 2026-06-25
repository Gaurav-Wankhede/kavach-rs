use super::advise;

#[test]
fn roadmap_without_exec_prompt_is_nudged() {
    assert!(advise("roadmap", None).is_some());
}

#[test]
fn roadmap_with_blank_exec_prompt_is_nudged() {
    assert!(advise("roadmap", Some("  \n ")).is_some());
}

#[test]
fn roadmap_with_a_real_prompt_is_silent() {
    assert!(advise("roadmap", Some("ROLE: ...")).is_none());
}

#[test]
fn non_roadmap_category_is_silent() {
    assert!(advise("decision", None).is_none());
}
