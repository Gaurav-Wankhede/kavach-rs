//! Pure-function proofs for the relevance reorder of runnable kanban cards.
use super::{RankableCard, rank_cards_by_relevance};

fn card(key: &str, status: &str) -> RankableCard {
    RankableCard {
        key: key.to_owned(),
        title: format!("title for {key}"),
        status: status.to_owned(),
        category: "roadmap".to_owned(),
    }
}

#[test]
fn empty_focus_preserves_priority_order() {
    // No brain hits (session-start / empty prompt) ⇒ input order is kept,
    // truncated to limit. Mirrors the whole-board fallback.
    let cards = vec![
        card("a", "todo"),
        card("b", "in_progress"),
        card("c", "todo"),
    ];
    let out = rank_cards_by_relevance(cards, &[], 6);
    let keys: Vec<&str> = out.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys, ["a", "b", "c"]);
}

#[test]
fn ranked_cards_lead_in_hit_order() {
    // brain.think hit ids are bare keys or qnames ending in the key. Ranked
    // cards come FIRST, in hit order; un-hit cards keep relative order after.
    let cards = vec![card("a", "todo"), card("b", "todo"), card("c", "todo")];
    let hits = vec!["c".to_owned(), "a".to_owned()];
    let out = rank_cards_by_relevance(cards, &hits, 6);
    let keys: Vec<&str> = out.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(
        keys,
        ["c", "a", "b"],
        "hit cards lead in hit order, rest follow"
    );
}

#[test]
fn qualified_hit_id_matches_bare_key() {
    // brain emits `roadmap.<key>` / `<project>/roadmap/<key>` — the suffix key
    // must still match the card.
    let cards = vec![card("alpha", "todo"), card("beta", "todo")];
    let hits = vec!["kavach-rs/roadmap/beta".to_owned()];
    let out = rank_cards_by_relevance(cards, &hits, 6);
    assert_eq!(out[0].key, "beta", "qualified id matched bare key");
}

#[test]
fn limit_truncates_after_ranking() {
    let cards = vec![card("a", "todo"), card("b", "todo"), card("c", "todo")];
    let hits = vec!["b".to_owned()];
    let out = rank_cards_by_relevance(cards, &hits, 2);
    let keys: Vec<&str> = out.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys, ["b", "a"], "ranked first, then truncate to 2");
}

#[test]
fn dotted_hit_prefix_matches_key() {
    // `roadmap.unit.foo` hit ⇒ matches card key `unit.foo`.
    let cards = vec![card("unit.foo", "todo")];
    let hits = vec!["roadmap.unit.foo".to_owned()];
    let out = rank_cards_by_relevance(cards, &hits, 6);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].key, "unit.foo");
}
