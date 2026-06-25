use super::blocked;

#[test]
fn new_roadmap_without_prompt_is_blocked() {
    assert!(blocked("roadmap", true, None).is_some());
}

#[test]
fn new_roadmap_with_blank_prompt_is_blocked() {
    assert!(blocked("roadmap", true, Some("   ")).is_some());
}

#[test]
fn new_roadmap_with_prompt_passes() {
    assert!(blocked("roadmap", true, Some("ROLE: do X")).is_none());
}

#[test]
fn update_without_prompt_passes() {
    assert!(blocked("roadmap", false, None).is_none());
}

#[test]
fn non_roadmap_category_passes() {
    assert!(blocked("decision", true, None).is_none());
    assert!(blocked("research", true, None).is_none());
}
