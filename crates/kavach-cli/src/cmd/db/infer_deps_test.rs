use super::{Card, already_declares, append_dep_line, infer, parse_seq};

/// Build a `Card` from just a key (title/content irrelevant to inference).
fn card(key: &str) -> Card {
    Card {
        seq: parse_seq(key),
        key: key.to_owned(),
        title: String::new(),
        content: String::new(),
    }
}

/// Collect inferred edges as `(card, prereq)` pairs, sorted for stable asserts.
fn edges_of(keys: &[&str]) -> Vec<(String, String)> {
    let cards: Vec<Card> = keys.iter().map(|k| card(k)).collect();
    let mut e: Vec<(String, String)> = infer(&cards)
        .into_iter()
        .map(|edge| (edge.card, edge.prereq))
        .collect();
    e.sort();
    e
}

#[test]
fn parses_letter_token_sequence() {
    let s = parse_seq("unit.harness-rl.p8-held-out").expect("p8 parses");
    assert_eq!(s.namespace, "unit.harness-rl");
    assert_eq!(s.token, "p");
    assert_eq!(s.n, 8);
}

#[test]
fn parses_word_token_before_bare_p() {
    // `phase` must win over the single-letter `p` (longest-first ordering).
    let s = parse_seq("unit.x.phase2").expect("phase parses");
    assert_eq!(s.token, "phase");
    assert_eq!(s.n, 2);
}

#[test]
fn parses_bare_trailing_number() {
    let s = parse_seq("unit.foo.3").expect("bare number parses");
    assert_eq!(s.token, "");
    assert_eq!(s.n, 3);
    assert_eq!(s.namespace, "unit.foo");
}

#[test]
fn singleton_without_dot_has_no_seq() {
    assert!(parse_seq("standalone-card").is_none());
}

#[test]
fn final_segment_not_a_sequence_has_no_seq() {
    assert!(parse_seq("unit.kanban-dag-native-relate").is_none());
}

#[test]
fn links_consecutive_same_namespace_tokens() {
    let e = edges_of(&[
        "unit.loop-eng-injection.f3-loop-goal",
        "unit.loop-eng-injection.f4-skill",
        "unit.loop-eng-injection.f5-concept",
    ]);
    assert_eq!(
        e,
        vec![
            (
                "unit.loop-eng-injection.f4-skill".to_owned(),
                "unit.loop-eng-injection.f3-loop-goal".to_owned()
            ),
            (
                "unit.loop-eng-injection.f5-concept".to_owned(),
                "unit.loop-eng-injection.f4-skill".to_owned()
            ),
        ]
    );
}

#[test]
fn first_in_sequence_has_no_predecessor() {
    // p7 with no p6 present -> no edge (predecessor must EXIST).
    let e = edges_of(&["unit.harness-rl.p7", "unit.harness-rl.p8"]);
    assert_eq!(
        e,
        vec![(
            "unit.harness-rl.p8".to_owned(),
            "unit.harness-rl.p7".to_owned()
        )]
    );
}

#[test]
fn different_namespaces_do_not_link() {
    let e = edges_of(&["unit.a.p1", "unit.b.p2"]);
    assert!(e.is_empty());
}

#[test]
fn different_tokens_same_namespace_do_not_link() {
    // f2 and p1 share a namespace but not a token kind -> no edge.
    let e = edges_of(&["unit.a.p1", "unit.a.f2"]);
    assert!(e.is_empty());
}

#[test]
fn singletons_produce_no_edges() {
    let e = edges_of(&[
        "unit.daemon-restart-race-free",
        "unit.kanban-dag-native-relate",
        "demo.badge-proof-blocked-card",
    ]);
    assert!(e.is_empty());
}

#[test]
fn already_declares_matches_gui_parse() {
    // Mirrors kanban::deps::declared_deps: comma/space-separated keys.
    assert!(already_declares("DEPENDS_ON: unit.a.p6\n", "unit.a.p6"));
    assert!(already_declares("body\nDEPENDS_ON: x, unit.a.p6, y", "unit.a.p6"));
    assert!(!already_declares("DEPENDS_ON: unit.a.p5", "unit.a.p6"));
    assert!(!already_declares("no deps here", "unit.a.p6"));
    // A substring must NOT false-match (p6 vs p60).
    assert!(!already_declares("DEPENDS_ON: unit.a.p60", "unit.a.p6"));
}

#[test]
fn append_dep_line_is_parseable_and_separated() {
    // Appended line must be re-detected by already_declares (round-trip).
    let out = append_dep_line("existing body", "unit.a.p6");
    assert!(out.contains("DEPENDS_ON: unit.a.p6"));
    assert!(already_declares(&out, "unit.a.p6"));
    // No body -> no leading blank line; trailing-newline body -> no double sep.
    assert_eq!(append_dep_line("", "k"), "DEPENDS_ON: k\n");
    assert_eq!(append_dep_line("a\n", "k"), "a\nDEPENDS_ON: k\n");
    assert_eq!(append_dep_line("a", "k"), "a\nDEPENDS_ON: k\n");
}

#[test]
fn ambiguous_predecessor_is_skipped() {
    // Two cards collide on (ns, token, n=1); the n=2 card's predecessor is
    // ambiguous -> fail-safe skip rather than guessing.
    let e = edges_of(&["unit.a.p1", "unit.a.p1-dup", "unit.a.p2"]);
    // p1 and p1-dup both parse to (unit.a, p, 1); p2 -> two predecessors -> skip.
    assert!(e.is_empty());
}
