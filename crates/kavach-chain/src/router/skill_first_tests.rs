use super::*;

#[test]
fn test_router_keyword_trigger() {
    let router = SkillFirstRouter::new();
    router.register_skill_trigger("rust", "backend");
    let d = router.route("implement", &["rust"]);
    assert!(d.use_skill);
    assert_eq!(d.skill_name, "backend");
}

#[test]
fn test_router_default() {
    let router = SkillFirstRouter::new();
    let d = router.route("hello", &[]);
    assert!(!d.use_skill);
    assert_eq!(d.agent_name, "backend-engineer");
}
