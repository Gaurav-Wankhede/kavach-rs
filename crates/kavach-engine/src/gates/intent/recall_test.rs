#[test]
fn test_keep_hit_filters_invalid_categories() {
    assert!(!keep_hit("hacked:evil"), "hacked prefix must be rejected");
    assert!(!keep_hit("malicious:data"), "malicious prefix must be rejected");
    assert!(!keep_hit("no_colon"), "id without colon must be rejected");
}

#[test]
fn test_keep_hit_accepts_valid_categories() {
    assert!(keep_hit("decision:foo"), "decision is valid");
    assert!(keep_hit("research:bar"), "research is valid");
    assert!(keep_hit("pattern:baz"), "pattern is valid");
    assert!(keep_hit("proposal:qux"), "proposal is valid");
    assert!(keep_hit("roadmap:quux"), "roadmap is valid");
    assert!(keep_hit("app_spec:corge"), "app_spec is valid");
}
