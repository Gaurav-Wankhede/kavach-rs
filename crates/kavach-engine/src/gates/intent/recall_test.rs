use super::*;

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

#[test]
fn test_recall_block_filters_invalid_hits() {
    let hits = vec![
        kavach_surreal::BrainHit {
            id: "hacked:evil".to_string(),
            score: 0.9,
        },
        kavach_surreal::BrainHit {
            id: "decision:foo".to_string(),
            score: 0.8,
        },
        kavach_surreal::BrainHit {
            id: "no_colon".to_string(),
            score: 0.7,
        },
    ];

    let mut block = String::from("[RECALL] prior memory relevant to this prompt (RRF-ranked):\n");
    for hit in &hits {
        if keep_hit(&hit.id) {
            block.push_str("  - ");
            block.push_str(&hit.id);
            block.push('\n');
        }
    }

    assert!(
        block.contains("decision:foo"),
        "valid hit 'decision:foo' must be included"
    );
    assert!(
        !block.contains("hacked:evil"),
        "invalid hit 'hacked:evil' must be excluded"
    );
    assert!(
        !block.contains("no_colon"),
        "malformed hit 'no_colon' must be excluded"
    );
}
